use super::super::*;

use std::sync::mpsc::*;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[allow(dead_code)]
struct BlockHook {
	hook: Arc<Mutex<Fn(u64, u64)>>,
}

unsafe impl Send for BlockHook { }

#[allow(dead_code)]
struct CodeHook {
	base: u64,
	hook: Arc<Mutex<Fn(u64, u64)>>,
}

unsafe impl Send for CodeHook { }

pub struct RegisterFile {
	bytes: [u8;4096],
	pc: u64,
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

	pub fn read_u32(&mut self, reg: u32) -> u32 {
		let reg_off = (reg as usize) * 4;

		((self.bytes[reg_off + 0] as u32) >>  0) |
		((self.bytes[reg_off + 1] as u32) >>  8) |
		((self.bytes[reg_off + 2] as u32) >> 16) |
		((self.bytes[reg_off + 3] as u32) >> 24)
	}

	pub fn set_pc(&mut self, value: u64) {
		self.pc = value
	}
}

enum Message {
	Shutdown(Arc<(Mutex<bool>, Condvar)>),
	FsbUpdateOp(mem::BusMatrixUpdateOp),
	SetReg(CpuReg, u64),
	AddBlockHookAll(BlockHook),
	AddCodeHookSingle(CodeHook),
	ExecuteRange(u64, u64),
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
}

impl Cpu for FrontEnd {
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error> {
		let _ = self.tx.send(Message::ExecuteRange(base, end));

		Ok(ExitReason::PcOutOfRange(end))
	}

	#[allow(unused_variables)]
	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::get_reg"))
	}

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error> {
		let _ = self.tx.send(Message::SetReg(reg, value));

		Ok(())
	}

	fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error> {
		let _ = self.tx.send(Message::AddBlockHookAll(BlockHook{hook: hook}));

		Ok(())
	}

	fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error> {
		let _ = self.tx.send(Message::AddCodeHookSingle(CodeHook{
			base: base,
			hook: hook
		}));

		Ok(())
	}

	fn shutdown(&mut self) {
		let remote_pair = Arc::new((Mutex::new(false), Condvar::new()));
		let local_pair = remote_pair.clone();

		let _ = self.tx.send(Message::Shutdown(remote_pair));

		println!("Sent shutdown message");

		let &(ref lock, ref cvar) = &*local_pair;
		let mut shutdown = lock.lock().unwrap();
		while !*shutdown {
			shutdown = cvar.wait(shutdown).unwrap();
		}
	}
}

#[derive(Debug, Eq, PartialEq)]
enum ExecutionState {
	Paused,
	WhileInRange(u64, u64),
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
			Message::Shutdown(arc) => {
				let &(ref lock, ref cvar) = &*arc;
				let mut started = lock.lock().unwrap();
				*started = true;
				cvar.notify_one();

				return false;
			},

			Message::FsbUpdateOp(update_op) => {
				self.fsb.apply_update_op(update_op);
			},

			Message::SetReg(reg, value) => {
				self.translator.set_reg(&mut self.registers, reg, value).unwrap();
			},

			Message::AddBlockHookAll(hook) => {
				self.hooks_on_all.push(hook);
			},

			Message::AddCodeHookSingle(hook) => {
				self.code_hooks_on_single.push(hook);
			},

			Message::ExecuteRange(base, end) => {
				self.execution_state = ExecutionState::WhileInRange(base, end);
			},
		}

		true
	}

	fn single_step(&mut self) {
		let value = self.registers.read_u32(1);
		self.registers.write_u32(1, value | 0x3456);
		self.registers.pc += 4;

		println!("Single step");
	}

	fn execute(&mut self) {
		let mut running = true;

		while running {
			running = match self.execution_state {
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

				ExecutionState::WhileInRange(base, end) => {
					self.single_step();

					if self.registers.pc < base || self.registers.pc >= end {
						self.execution_state = ExecutionState::Paused;
					}

					let msg = match self.rx.try_recv() {
						Ok(msg) => msg,
						Err(err) => {
							println!("Exiting cpu thread because {:?}", err);
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

	fsb.add_child_matrix(Box::new(move |update_op| {
		mem_update_channel.send(Message::FsbUpdateOp(update_op)).unwrap();
	}));

	thread::spawn(move || {
		let mut backend = Backend::new(rx, translator);

		backend.execute();
	});

	Ok(Box::new(FrontEnd::new(tx)))
}

