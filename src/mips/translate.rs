use super::Arch;

use super::super::iisa;

use std::io;

extern crate opcode;

#[derive(Default)]
#[allow(dead_code)]
pub struct MipsTranslator {
	arch: Arch,
	big_endian: bool,
}

#[derive(PartialEq)]
enum BaseIsa {
	Mips32,
	Mips64,
}

fn isa_for_arch(arch: &Arch) -> BaseIsa {
	match *arch {
		Arch::R2000  => BaseIsa::Mips32,
		Arch::Sys161 => BaseIsa::Mips32,
		Arch::VR4300 => BaseIsa::Mips64,
	}
}

fn dest_gpr(gpr_num: u8) -> iisa::R {
	match gpr_num {
		0 => iisa::R::Discard,
		_ => iisa::R::W(gpr_num as u16),
	}
}

fn decode_mips32(arch: &Arch, base: u64, op: &opcode::mips::Op) -> Result<Vec<iisa::Instr>, io::Error> {
	let result = match *op {
		opcode::mips::Op::RtU16(opcode::mips::Mne::Lui, opcode::mips::Reg::Gpr(gpr), imm) => {
			iisa::Op::Ld(iisa::DstSrc{dst: dest_gpr(gpr), src: iisa::Src::ImmU32((imm as u32) << 16)})
		},

		_ => {
			return Err(io::Error::new(io::ErrorKind::Other, "mips32 decode Unimplemented"));
		},
	};

	Ok(vec!(iisa::Instr{op: result, pred: iisa::Pred::None, exc: 0, size: 4}))
}

#[allow(unused_variables)]
fn decode_mips64(arch: &Arch, base: u64, buffer: &[u8]) -> Result<Vec<iisa::Instr>, io::Error> {
	Err(io::Error::new(io::ErrorKind::Other, "mips64 decode Unimplemented"))
}

fn uarch_opts_for_arch(arch: &Arch) -> Option<&'static opcode::mips::UarchInfo> {
	match *arch {
		Arch::R2000  => Some(opcode::mips::uarch_info_for_uarch(opcode::mips::Uarch::LsiR2000)),
		Arch::Sys161 => Some(opcode::mips::uarch_info_for_uarch(opcode::mips::Uarch::HarvardMips161)),
		_            => None,
	}
}

impl MipsTranslator {
	pub fn decode(&self, base: u64, buffer: &[u8]) -> Result<Vec<iisa::Instr>, io::Error> {
		if (base % 4) != 0 {
			return Err(io::Error::new(io::ErrorKind::Other, "Buffer not aligned on instruction boundary"));
		}
		if buffer.len() < 4 {
			return Err(io::Error::new(io::ErrorKind::Other, "Buffer not large enough"));
		}

		let decode_opts = opcode::mips::DecodeOptions{ decode_pseudo_ops: false, big_endian: self.big_endian };
		let uarch_info = uarch_opts_for_arch(&self.arch).unwrap();
		let instr_word = if self.big_endian {
			((buffer[0] as u32) << 24) |
			((buffer[1] as u32) << 16) |
			((buffer[2] as u32) << 8 ) |
			((buffer[3] as u32) << 0 )
		} else {
			((buffer[0] as u32) << 0 ) |
			((buffer[1] as u32) << 8 ) |
			((buffer[2] as u32) << 16) |
			((buffer[3] as u32) << 24)
		};

		let op = opcode::mips::decode(instr_word, base, uarch_info, &decode_opts).unwrap();

		match isa_for_arch(&self.arch) {
			BaseIsa::Mips32 => decode_mips32(&self.arch, base, &op),
			BaseIsa::Mips64 => decode_mips64(&self.arch, base, buffer),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::super::super::iisa::{DstSrc, Instr, Op, Pred, R, Src};
	use super::super::Arch;
	use super::MipsTranslator;

	enum TestCase {
		Normal{ instr: u32, translated: Op },
	}

	static MIPS32_TESTCASES: [TestCase; 2] = [
		// 3c00abcd : lui     zero,0xabcd     | ld      discard,0x80720000
		TestCase::Normal{ instr: 0x3c00abcd, translated: Op::Ld(DstSrc{dst: R::Discard, src: Src::ImmU32(0xABCD0000)}) },

		// 3c1c8072 : lui     gp,0x8072       | ld      w28,0x80720000
		TestCase::Normal{ instr: 0x3c1c8072, translated: Op::Ld(DstSrc{dst: R::W(28), src: Src::ImmU32(0x80720000)}) },
	];

	#[test]
	fn translate_r2000() {
		let mut translator = MipsTranslator{ arch: Arch::R2000, big_endian: true };

		for test_case in MIPS32_TESTCASES.iter() {
			match test_case {
				&TestCase::Normal{instr, ref translated} => {
					let buffer_be: [u8; 4] = [
						(instr >> 24) as u8,
						(instr >> 16) as u8,
						(instr >>  8) as u8,
						(instr >>  0) as u8,
					];

					translator.big_endian = true;
					let iisa = translator.decode(0, &buffer_be).unwrap();
					assert_eq!(iisa, vec!(Instr{op: translated.clone(), pred: Pred::None, exc: 0, size: 4}));
				},
			}
		}
	}
}

