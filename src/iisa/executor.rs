use super::super::*;

use super::RegisterFile;
use super::Translator;

use std::mem::transmute;
use std::sync::mpsc::*;
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(dead_code)]
struct BlockHook {
	hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>,
}

unsafe impl Send for BlockHook { }

#[allow(dead_code)]
struct CodeHook {
	base: u64,
	hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>,
}

unsafe impl Send for CodeHook { }

enum Message {
	Shutdown(Promise<()>),
	FsbUpdateOp(mem::BusMatrixUpdateOp, Promise<()>),
	SetReg(CpuReg, u64, Promise<()>),
	GetReg(CpuReg, Promise<u64>),
	AddBlockHookAll(BlockHook, Promise<()>),
	AddCodeHookSingle(CodeHook, Promise<()>),
	Execute(Promise<ExitReason>),
}

struct FrontEnd {
	tx: Sender<Message>,
}

impl FrontEnd {
	fn new(tx: Sender<Message>) -> FrontEnd {
		FrontEnd {
			tx: tx,
		}
	}
}

impl Cpu for FrontEnd {
	fn execute(&mut self) -> Result<ExitReason, Error> {
		let mut promise = Promise::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::Execute(promise));

		future.wait()
	}

	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error> {
		let mut promise = Promise::<u64>::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::GetReg(reg, promise));

		future.wait()
	}

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error> {
		let mut promise = Promise::<()>::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::SetReg(reg, value, promise));

		future.wait()
	}

	fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error> {
		let mut promise = Promise::<()>::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::AddBlockHookAll(BlockHook{hook: hook}, promise));

		future.wait()
	}

	fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error> {
		let mut promise = Promise::<()>::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::AddCodeHookSingle(CodeHook{
			base: base,
			hook: hook
		}, promise));

		future.wait()
	}

	fn shutdown(&mut self) {
		let mut promise = Promise::<()>::new();
		let future = promise.get_future();

		let _ = self.tx.send(Message::Shutdown(promise));

		let _ = future.wait();
	}
}

const PAGE_SIZE: usize = 4096;

struct Page<'a> {
	base: u64,
	data: &'a [u8;PAGE_SIZE],
}

impl<'a> Page<'a> {
	fn new(base: u64, data: &'a [u8;PAGE_SIZE]) -> Page {
		Page {
			base: base,
			data: data,
		}
	}

	fn single_step(&self, reg: &mut RegisterFile, translator: &Translator) -> Result<(), Error> {
		let offset = (reg.pc - self.base) as usize;

		if offset > (PAGE_SIZE - 1) {
			return Err(Error::InvalidPC);
		}

		let instrs = try!(translator.decode(reg.pc, &self.data[offset..]));

		try!(iisa::interpret_op_list(&instrs, reg));

		Ok(())
	}
}

#[derive(Clone)]
enum ExecutionState {
	Paused,
	Executing(Promise<ExitReason>),
}

struct Backend<T: Send> {
	rx: Receiver<Message>,
	translator: T,
	fsb: mem::BusMatrix,
	registers: RegisterFile,
	hooks_on_all: Vec<BlockHook>,
	code_hooks_on_single: Vec<CodeHook>,
	execution_state: ExecutionState,
}

impl<T: Send+Clone+Translator> Backend<T> {
	fn new(rx: Receiver<Message>, translator: T) -> Backend<T> {
		Backend {
			rx:                   rx,
			translator:           translator,
			fsb:                  Default::default(),
			registers:            RegisterFile::new(),
			hooks_on_all:         Vec::new(),
			code_hooks_on_single: Vec::new(),
			execution_state:      ExecutionState::Paused,
		}
	}

	fn process_message(&mut self, msg: Message) -> bool {
		match msg {
			Message::Shutdown(mut promise) => {
				promise.signal(Ok(()));

				return false;
			},

			Message::FsbUpdateOp(update_op, mut promise) => {
				self.fsb.apply_update_op(update_op);

				promise.signal(Ok(()));
			},

			Message::GetReg(reg, mut promise) => {
				promise.signal(self.translator.get_reg(&self.registers, reg))
			},

			Message::SetReg(reg, value, mut promise) => {
				promise.signal(self.translator.set_reg(&mut self.registers, reg, value))
			},

			Message::AddBlockHookAll(hook, mut promise) => {
				self.hooks_on_all.push(hook);

				promise.signal(Ok(()));
			},

			Message::AddCodeHookSingle(hook, mut promise) => {
				self.code_hooks_on_single.push(hook);

				promise.signal(Ok(()));
			},

			Message::Execute(promise) => {
				self.execution_state = ExecutionState::Executing(promise);
			},
		}

		true
	}

	fn single_step(&mut self) -> Result<(), Error> {
		let page_virt_base = self.registers.pc & !((PAGE_SIZE as u64) - 1);
		let page_phys_base = match self.translator.virtual_to_phys(&mut self.registers, page_virt_base) {
			Some(virt) => virt,
			None => return Err(Error::VirtualAddrNotMappable(page_virt_base)),
		};
		let page_mem = match self.fsb.find_range(page_phys_base, PAGE_SIZE) {
			Ok(raw_ptr) => {
				unsafe { transmute(raw_ptr) }
			},
			Err(err) => {
				return Err(err);
			},
		};
		Page::new(page_virt_base, page_mem).single_step(&mut self.registers, &self.translator)
	}

	fn execute(&mut self) {
		let mut running = true;

		while running {
			let cur_state = self.execution_state.clone();
			running = match cur_state {
				ExecutionState::Paused => {
					let msg = match self.rx.recv() {
						Ok(msg) => msg,
						Err(err) => {
							println!("Exiting cpu thread because {:?}", err);
							return;
						},
					};

					self.process_message(msg)
				},

				ExecutionState::Executing(mut promise) => {
					promise.signal(match self.single_step() {
						Ok(()) => Ok(ExitReason::CodeHookSignalledStop),
						Err(err) => Err(err),
					});

					self.execution_state = ExecutionState::Paused;

					let msg = match self.rx.try_recv() {
						Ok(msg) => msg,
						Err(TryRecvError::Empty) => {
							continue;
						},
						Err(TryRecvError::Disconnected) => {
							return;
						},
					};

					self.process_message(msg)
				},
			}
		}
	}
}

pub fn executor<T: 'static+Send+Clone+Translator>(translator: T, fsb: &mut mem::BusMatrix) -> Result<Box<Cpu>, Error> {
	let (tx, rx) = channel::<Message>();

	let mem_update_channel = tx.clone();

	thread::spawn(move || {
		let mut backend = Backend::new(rx, translator);

		backend.execute();
	});

	fsb.add_child_matrix(Box::new(move |update_op| {
		let mut promise = Promise::<()>::new();
		let future = promise.get_future();

		let _ = mem_update_channel.send(Message::FsbUpdateOp(update_op, promise));

		let _ = future.wait();
	}));

	Ok(Box::new(FrontEnd::new(tx)))
}

