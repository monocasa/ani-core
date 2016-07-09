use super::super::*;

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

pub struct RegisterFile {
	bytes: [u8;4096],
	pub pc: u64,
}

impl RegisterFile {
	fn new() -> RegisterFile {
		RegisterFile {
			bytes: [0; 4096],
			pc:    0,
		}
	}

	pub fn write_u32(&mut self, reg: u32, value: u32) {
		let reg_off: usize = (reg as usize) * 4;
		self.bytes[reg_off + 0] = (value >>  0) as u8;
		self.bytes[reg_off + 1] = (value >>  8) as u8;
		self.bytes[reg_off + 2] = (value >> 16) as u8;
		self.bytes[reg_off + 3] = (value >> 24) as u8;
	}

	pub fn read_u32(&self, reg: u32) -> u32 {
		let reg_off = (reg as usize) * 4;

		((self.bytes[reg_off + 0] as u32) <<  0) |
		((self.bytes[reg_off + 1] as u32) <<  8) |
		((self.bytes[reg_off + 2] as u32) << 16) |
		((self.bytes[reg_off + 3] as u32) << 24)
	}
}

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

pub trait Translator {
	fn set_reg(&mut self, registers: &mut RegisterFile, reg: CpuReg, value: u64) -> Result<(), Error>;
	fn get_reg(&self, registers: &RegisterFile, reg: CpuReg) -> Result<u64, Error>;
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

	fn single_step(&mut self) {
		let value = self.registers.read_u32(1);
		self.registers.write_u32(1, value | 0x3456);
		self.registers.pc += 4;
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
					self.single_step();

					promise.signal(Ok(ExitReason::CodeHookSignalledStop));
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

