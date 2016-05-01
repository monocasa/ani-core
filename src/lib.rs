#![cfg_attr(feature = "assignment_operators", feature(augmented_assignments, op_assign_traits))]
#[macro_use]
extern crate bitflags;

pub mod iisa;
pub mod mem;
pub mod mips;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

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

#[derive(Debug, PartialEq)]
pub enum ExitReason {
	PcOutOfRange(u64),
}

#[derive(Debug)]
pub enum CpuReg {
	Pc,
	CpuSpecific(u32),
}

#[derive(Debug)]
pub enum Error {
	Unimplemented(&'static str),

	MemAllocation,
	UnableToFindRange,

	InvalidCpuCookie,

	OptNotSupported(CpuOpt),
	UnimplementedArchitecture,

	GetRegUnknownReg(CpuReg),

	SetRegValueOutOfRange(CpuReg, u64),
	SetRegUnknownReg(CpuReg, u64),
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

	pub fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error> {
		for (_, cpu) in self.cpus.iter_mut() {
			try!(cpu.add_block_hook_all(hook.clone()));
		}

		Ok(())
	}

	pub fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error> {
		for(_, cpu) in self.cpus.iter_mut() {
			try!(cpu.add_code_hook_single(base, hook.clone()));
		}

		Ok(())
	}

	pub fn execute_cpu_range(&mut self, cpu_cookie: &CpuCookie, base: u64, end: u64) -> Result<ExitReason, Error> {
		try!(self.get_cpu(cpu_cookie)).execute_range(base, end)
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
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error>;

	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error>;

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error>;

	fn add_block_hook_all(&mut self, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error>;
	fn add_code_hook_single(&mut self, base: u64, hook: Arc<Mutex<Fn(u64, u64)>>) -> Result<(), Error>;

	fn shutdown(&mut self);
}

