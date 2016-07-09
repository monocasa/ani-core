use super::{Cpu,
            CPU_ENDIAN_BIG,
            CpuOpt,
            CpuReg,
            Error};

use super::iisa;
use super::mem;

use std::sync::{Arc, Mutex};

pub mod translate;

#[derive(Clone, PartialEq)]
pub enum Arch {
	R2000,
	Sys161,
	VR4300,
	Mips4Kc,
}

impl Default for Arch {
	fn default() -> Arch {
		Arch::R2000
	}
}

pub const REG_AT: CpuReg = CpuReg::CpuSpecific(1);

#[allow(dead_code)]
enum HookRange {
	All,
}

#[allow(dead_code)]
struct BlockHook {
	range: HookRange,
	hook: Arc<Mutex<Fn(u64, u64)>>,
}

#[allow(dead_code)]
struct CodeHook {
	base: u64,
	hook: Arc<Mutex<Fn(u64, u64)>>,
}

pub fn mips_cpu_factory(opts: CpuOpt, arch: Arch, fsb: &mut mem::BusMatrix) -> Result<Box<Cpu>, Error> {
	let translator = translate::MipsTranslator{ arch: arch, big_endian: (opts & CPU_ENDIAN_BIG) == CPU_ENDIAN_BIG };

	iisa::executor::executor(translator, fsb)
}

