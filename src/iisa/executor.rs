use super::super::*;

use super::super::mem::{BusMatrix, BusMatrixUpdateOp};

use std::sync::mpsc::*;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

enum Message {
	Shutdown(Arc<(Mutex<bool>, Condvar)>),
	FsbUpdateOp(mem::BusMatrixUpdateOp),
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
	#[allow(unused_variables)]
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::execute_range"))
	}

	#[allow(unused_variables)]
	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::get_reg"))
	}

	#[allow(unused_variables)]
	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::set_reg"))
	}

	#[allow(unused_variables)]
	fn add_block_hook_all(&mut self, hook: Arc<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::add_block_hook_all"))
	}

	#[allow(unused_variables)]
	fn add_code_hook_single(&mut self, base: u64, hook: Arc<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented("iisa::executor::FrontEnd::add_code_hook_single"))
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
}

impl<T: Send> Backend<T> {
	fn new(rx: Receiver<Message>, translator: T) -> Backend<T> {
		Backend {
			rx:         rx,
			translator: translator,
			fsb:        Default::default(),
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
			}
		}
	}
}

pub fn executor<T: 'static+Send+Clone>(translator: T, fsb: &mut mem::BusMatrix) -> Result<Box<Cpu>, Error> {
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

