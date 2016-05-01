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
	#[allow(unused_variables)]
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::execute_range"))
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

struct Backend<T: Send> {
	rx: Receiver<Message>,
	translator: T,
	fsb: mem::BusMatrix,
	registers: RegisterFile,
	hooks_on_all: Vec<BlockHook>,
	code_hooks_on_single: Vec<CodeHook>,
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
		}
	}

	fn execute(&mut self) {
		let mut running = true;

		while running {
			let msg = match self.rx.recv() {
				Ok(msg) => msg,
				Err(err) => {
					println!("Exiting cpu thread because {:?}", err);
					return;
				},
			};

			match msg {
				Message::Shutdown(arc) => {
					running = false;
					let &(ref lock, ref cvar) = &*arc;
					let mut started = lock.lock().unwrap();
					*started = true;
					cvar.notify_one();
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

