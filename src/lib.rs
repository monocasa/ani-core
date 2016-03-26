#![feature(const_fn)]

#![cfg_attr(feature = "assignment_operators", feature(augmented_assignments, op_assign_traits))]
#[macro_use]
extern crate bitflags;

pub mod iisa;
pub mod mem;
pub mod mips;

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

#[allow(dead_code)]
pub struct System {
	fsb: mem::MemMap,
	cpus: Vec<Box<Cpu>>,
}

#[allow(dead_code)]
pub struct CpuCookie {
	handle: usize,
}

pub enum Arch {
	Mips(mips::Arch),
}

impl System {
	pub fn new() -> System {
		System {
			fsb: Default::default(),
			cpus: Vec::new(),
		}
	}

	pub fn add_mappable_range(&mut self, prot: MemProt, base: u64, size: u64) -> Result<(), Error> {
		self.fsb.add_mappable_range(base, size, prot)
	}

	#[allow(unused_variables)]
	pub fn register_cpu(&mut self, opts: CpuOpt, arch: Arch) -> Result<CpuCookie, Error> {
		Err(Error::Unimplemented("register_cpu"))
	}

	#[allow(unused_variables)]
	pub fn set_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg, value: u64) -> Result<(), Error> {
		Err(Error::Unimplemented("set_cpu_reg"))
	}

	#[allow(unused_variables)]
	pub fn add_block_hook_all(&mut self, hook: Box<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented("add_block_hook_all"))
	}

	#[allow(unused_variables)]
	pub fn add_code_hook_single(&mut self, base: u64, hook: Box<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented("add_code_hook_single"))
	}

	#[allow(unused_variables)]
	pub fn execute_cpu_range(&mut self, cpu_cookie: &CpuCookie, base: u64, end: u64) -> Result<ExitReason, Error> {
		Err(Error::Unimplemented("execute_cpu_range"))
	}

	#[allow(unused_variables)]
	pub fn get_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg) -> Result<u64, Error> {
		Err(Error::Unimplemented("get_cpu_reg"))
	}

	pub fn set_range(&mut self, incoming: &[u8], base_addr: u64) -> Result<(), Error> {
		self.fsb.set_range(incoming, base_addr)
	}
}

pub trait Cpu {
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error>;

	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error>;

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error>;

	fn add_block_hook_all(&mut self, hook: Box<Fn(u64, u64)>) -> Result<(), Error>;
	fn add_code_hook_single(&mut self, base: u64, hook: Box<Fn(u64, u64)>) -> Result<(), Error>;
}

