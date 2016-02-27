#![feature(const_fn)]

#![cfg_attr(feature = "assignment_operators", feature(augmented_assignments, op_assign_traits))]
#[macro_use]
extern crate bitflags;

pub mod iisa;
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
	Unimplemented,

	InvalidCpuCookie,

	OptNotSupported(CpuOpt),
	UnimplementedArchitecture,

	GetRegUnknownReg(CpuReg),

	SetRegValueOutOfRange(CpuReg, u64),
	SetRegUnknownReg(CpuReg, u64),
}

#[allow(dead_code)]
enum MemSlotType {
	Mem,
	Mmio,
}

#[allow(dead_code)]
struct MemSlot {
	base: u64,
	size: u64,
	slot_type: MemSlotType,
	prot: MemProt,
}

#[allow(dead_code)]
pub struct System {
	mem_slots: Vec<MemSlot>,
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
			mem_slots: Vec::new(),
			cpus: Vec::new(),
		}
	}

	pub fn add_ram_region(&mut self, prot: MemProt, base: u64, size: u64) -> Result<(), Error> {
		self.mem_slots.push(MemSlot {
			base: base,
			size: size,
			slot_type: MemSlotType::Mem,
			prot: prot,
		});

		Ok(())
	}

	#[allow(unused_variables)]
	pub fn register_cpu(&mut self, opts: CpuOpt, arch: Arch) -> Result<CpuCookie, Error> {
		Ok(CpuCookie{handle: 0})
	}

	#[allow(unused_variables)]
	pub fn set_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg, value: u64) -> Result<(), Error> {
		Err(Error::Unimplemented)
	}

	#[allow(unused_variables)]
	pub fn add_block_hook_all(&mut self, hook: Box<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented)
	}

	#[allow(unused_variables)]
	pub fn add_code_hook_single(&mut self, base: u64, hook: Box<Fn(u64, u64)>) -> Result<(), Error> {
		Err(Error::Unimplemented)
	}

	#[allow(unused_variables)]
	pub fn execute_cpu_range(&mut self, cpu_cookie: &CpuCookie, base: u64, end: u64) -> Result<ExitReason, Error> {
		Err(Error::Unimplemented)
	}

	#[allow(unused_variables)]
	pub fn get_cpu_reg(&mut self, cpu_cookie: &CpuCookie, reg: CpuReg) -> Result<u64, Error> {
		Err(Error::Unimplemented)
	}

	#[allow(unused_variables)]
	pub fn write_range(&mut self, incoming: &[u8], base_addr: u64) -> Result<(), Error> {
		Ok(())
	}
}

pub trait Cpu {
	fn execute_range(&mut self, base: u64, end: u64) -> Result<ExitReason, Error>;

	fn get_reg(&self, reg: CpuReg) -> Result<u64, Error>;

	fn set_reg(&mut self, reg: CpuReg, value: u64) -> Result<(), Error>;

	fn add_block_hook_all(&mut self, hook: Box<Fn(u64, u64)>) -> Result<(), Error>;
	fn add_code_hook_single(&mut self, base: u64, hook: Box<Fn(u64, u64)>) -> Result<(), Error>;
}

