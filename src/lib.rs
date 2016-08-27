#![cfg_attr(feature = "assignment_operators", feature(augmented_assignments, op_assign_traits))]
#![feature(slice_patterns)]
#[macro_use]
extern crate bitflags;

pub mod iisa;
pub mod mem;
pub mod mips;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

bitflags! {
	flags MemProt: u8 {
		const PROT_READ  = 0b0001,
		const PROT_WRITE = 0b0010,
		const PROT_EXEC  = 0b0100,
		const PROT_RW    = PROT_READ.bits
		                 | PROT_WRITE.bits,
		const PROT_ALL   = PROT_READ.bits
		                 | PROT_WRITE.bits
		                 | PROT_EXEC.bits,
	}
}

bitflags! {
	flags CpuOpt: u8 {
		const CPU_ENDIAN_BIG    = 0b00000001,
		const CPU_ENDIAN_LITTLE = 0b00000000,
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExitReason {
	CodeHookSignalledStop
}

#[derive(Clone, Debug)]
pub enum CpuReg {
	Pc,
	CpuSpecific(u32),
}

#[derive(Clone, Debug)]
pub enum Error {
	Unimplemented(String),

	MemAllocation,
	UnableToFindRange(u64, usize),

	InvalidCpuCookie,

	OptNotSupported(CpuOpt),
	UnimplementedArchitecture,

	GetRegUnknownReg(CpuReg),

	SetRegValueOutOfRange(CpuReg, u64),
	SetRegUnknownReg(CpuReg, u64),

	InvalidPC,
	VirtualAddrNotMappable(u64),

	PromiseLost,
}

pub struct System {
	fsb: mem::BusMatrix,
	cpus: BTreeMap<usize, Box<Cpu>>,
	next_cpu_handle: usize,
}

pub struct CpuCookie {
	handle: usize,
}

pub enum Arch {
	Mips(mips::Arch),
}

pub enum TraceExitHint {
	ContinueExecution,
	StopExecution
}

pub struct Future<T> {
	rx: mpsc::Receiver<Result<T, Error>>,
}

impl<T> Future<T> {
	pub fn wait(self) -> Result<T, Error> {
		match self.rx.recv() {
			Ok(t) => t,
			Err(_) => Err(Error::PromiseLost),
		}
	}
}

#[derive(Clone)]
pub struct Promise<T: Clone> {
	future_channels: Vec<mpsc::Sender<Result<T, Error>>>,
}

impl<T: Clone> Promise<T> {
	fn new() -> Promise<T> {
		Promise {
			future_channels: Vec::new(),
		}
	}

	fn signal(&mut self, result: Result<T, Error>) {
		for future_channel in self.future_channels.iter() {
			let _ = future_channel.send(result.clone());
		}
	}

	fn get_future(&mut self) -> Future<T> {
		let (tx, rx) = mpsc::channel();

		self.future_channels.push(tx);

		Future {
			rx: rx,
		}
	}
}

fn create_cpu(opts: CpuOpt, arch: Arch, fsb: &mut mem::BusMatrix) -> Result<Box<Cpu>, Error> {
	match arch {
		Arch::Mips(mips_info) => {
			mips::mips_cpu_factory(opts, mips_info, fsb)
		},
	}
}

impl System {
	pub fn new() -> System {
		System {
			fsb: Default::default(),
			cpus: BTreeMap::new(),
			next_cpu_handle: 0,
		}
	}

	pub fn add_mappable_range(&mut self, prot: MemProt, base: u64, size: u64) -> Result<(), Error> {
		self.fsb.add_mappable_range(base, size, prot)
	}

	fn register_cpu_no_throw(&mut self, cpu: Box<Cpu>) -> CpuCookie {
		let this_handle = self.next_cpu_handle;

		self.next_cpu_handle += 1;

		let _ = self.cpus.insert(this_handle, cpu);

		CpuCookie{handle: this_handle}
	}

	pub fn register_cpu(&mut self, opts: CpuOpt, arch: Arch) -> Result<CpuCookie, Error> {
		let cpu = try!(create_cpu(opts, arch, &mut self.fsb));

		Ok(self.register_cpu_no_throw(cpu))
	}

	fn get_cpu(&mut self, cookie: &CpuCookie) -> Result<&mut Box<Cpu>, Error> {
		match self.cpus.get_mut(&cookie.handle) {
			Some(cpu) => Ok(cpu),
			None      => Err(Error::InvalidCpuCookie),
		}
	}

	pub fn set_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg, value: u64) -> Result<(), Error> {
		try!(self.get_cpu(cpu_cookie)).set_reg(reg, value)
	}

	pub fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error> {
		for (_, cpu) in self.cpus.iter_mut() {
			try!(cpu.add_block_hook_all(hook.clone()));
		}

		Ok(())
	}

	pub fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error> {
		for(_, cpu) in self.cpus.iter_mut() {
			try!(cpu.add_code_hook_single(base, hook.clone()));
		}

		Ok(())
	}

	pub fn execute(&mut self, cpu_cookie: &CpuCookie) -> Result<ExitReason, Error> {
		try!(self.get_cpu(cpu_cookie)).execute()
	}

	pub fn get_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg) -> Result<u64, Error> {
		try!(self.get_cpu(cpu_cookie)).get_reg(reg)
	}

	pub fn set_range(&mut self, incoming: &[u8], base_addr: u64) -> Result<(), Error> {
		self.fsb.set_range(incoming, base_addr)
	}
}

impl Drop for System {
	fn drop(&mut self) {
		for(_, cpu) in self.cpus.iter_mut() {
			cpu.shutdown();
		}
	}
}

pub trait Cpu {
	fn execute(&mut self) -> Result<ExitReason, Error>;

	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error>;

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error>;

	fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error>;
	fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64) -> TraceExitHint>>) -> Result<(), Error>;

	fn shutdown(&mut self);
}

