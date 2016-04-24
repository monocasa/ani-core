use super::Arch;

use super::super::iisa;

use std::io;

extern crate opcode;

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct MipsTranslator {
	pub arch: Arch,
	pub big_endian: bool,
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

fn src_gpr(gpr_num: u8) -> iisa::Src {
	match gpr_num {
		0 => iisa::Src::ImmU32(0),
		_ => iisa::Src::Reg(iisa::R::W(gpr_num as u16)),
	}
}

fn src_i16(imm: i16) -> iisa::Src {
	iisa::Src::ImmI16(imm)
}

fn src_u16(imm: u16) -> iisa::Src {
	iisa::Src::ImmU16(imm)
}

fn src_u32(imm: u32) -> iisa::Src {
	iisa::Src::ImmU32(imm)
}

fn sw_src(gpr_num: u8) -> iisa::Src {
	match gpr_num {
		0 => iisa::Src::ImmU32(0),
		_ => iisa::Src::Reg(iisa::R::W(gpr_num as u16)),
	}
}

#[allow(unused_variables)]
fn decode_mips32(arch: &Arch, base: u64, buffer: &[u8], big_endian: bool, in_delay_slot: bool) -> Result<Vec<iisa::Instr>, io::Error> {
	let decode_opts = opcode::mips::DecodeOptions{ decode_pseudo_ops: false, big_endian: big_endian };
	let uarch_info = uarch_opts_for_arch(arch).unwrap();

	let op = opcode::mips::decode_buf(buffer, base, uarch_info, &decode_opts).unwrap();

	let result = match op {
		opcode::mips::Op::RtRsI16(opcode::mips::Mne::Addiu,
		                          opcode::mips::Reg::Gpr(rt),
		                          opcode::mips::Reg::Gpr(rs),
		                          imm) => {
			iisa::Op::Add(iisa::DstSrcSrc{dst: dest_gpr(rt), src: [src_gpr(rs), src_i16(imm)]})
		},

		opcode::mips::Op::RtRsU16(opcode::mips::Mne::Ori,
		                          opcode::mips::Reg::Gpr(rt),
		                          opcode::mips::Reg::Gpr(rs),
		                          imm) => {
			iisa::Op::Or(iisa::DstSrcSrc{dst: dest_gpr(rt), src: [src_gpr(rs), src_u16(imm)]})
		},

		opcode::mips::Op::RdRsRt(opcode::mips::Mne::Addu,
		                         opcode::mips::Reg::Gpr(rd),
		                         opcode::mips::Reg::Gpr(rs),
		                         opcode::mips::Reg::Gpr(rt)) => {
			iisa::Op::Add(iisa::DstSrcSrc{dst: dest_gpr(rd), src: [src_gpr(rs), src_gpr(rt)]})
		},

		opcode::mips::Op::RsRtTarget(opcode::mips::Mne::Beq,
		                             opcode::mips::Reg::Gpr(rs),
		                             opcode::mips::Reg::Gpr(rt),
		                             opcode::AddrTarget::Relative(offset)) => {
			if in_delay_slot {
				return Err(io::Error::new(io::ErrorKind::Other, format!("Branch while in delay slot")));
			}

			let delay_slot_buffer = &buffer[4..];

			let other_instr = try!(decode_mips32(arch, base + 4, delay_slot_buffer, big_endian, true));

			let branch_target = (((base as i64) + offset + 4) as u64) & 0x00000000FFFFFFFFu64;

			let branch_instr = iisa::Op::B(iisa::Cond::Eq,
			                               iisa::SrcSrcTarget{src: [src_gpr(rs), src_gpr(rt)],
			                                                  target: iisa::Src::Addr(branch_target)} );

			return Ok(vec!( iisa::Instr{op: other_instr[0].op,  pred: iisa::Pred::None, exc: 1, size: 0},
			                iisa::Instr{op: branch_instr, pred: iisa::Pred::None, exc: 2, size: 8},), );
		},

		opcode::mips::Op::RtU16(opcode::mips::Mne::Lui, opcode::mips::Reg::Gpr(rt), imm) => {
			iisa::Op::Ld(iisa::DstSrc{dst: dest_gpr(rt), src: src_u32((imm as u32) << 16)})
		},

		opcode::mips::Op::RtOffsetBase(opcode::mips::Mne::Sw,
		                               opcode::mips::Reg::Gpr(rt),
		                               offset,
		                               opcode::mips::Reg::Gpr(base)) => {
			iisa::Op::Sw(iisa::SrcSrcSrc{src: [sw_src(rt), src_i16(offset), src_gpr(base)]})
		},

		_ => {
			return Err(io::Error::new(io::ErrorKind::Other, format!("mips32 decode Unimplemented {:?}", op)));
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
		match isa_for_arch(&self.arch) {
			BaseIsa::Mips32 => decode_mips32(&self.arch, base, buffer, self.big_endian, false),
			BaseIsa::Mips64 => decode_mips64(&self.arch, base, buffer),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::super::super::iisa::{Cond,
	                                DstSrc,
	                                DstSrcSrc,
	                                Instr,
	                                Op,
	                                Pred,
	                                R,
	                                Src,
	                                SrcSrcSrc,
	                                SrcSrcTarget};

	use super::super::Arch;
	use super::MipsTranslator;

	macro_rules! test_simple_r2000 {
		($func_name:ident, $instr:expr, $translated:expr) => (
			#[test]
			#[allow(non_snake_case)]
			fn $func_name() {
				let translator_be = MipsTranslator{ arch: Arch::R2000, big_endian: true };
				let translator_le = MipsTranslator{ arch: Arch::R2000, big_endian: false };

				let buffer_be: [u8; 4] = [
					($instr >> 24) as u8,
					($instr >> 16) as u8,
					($instr >>  8) as u8,
					($instr >>  0) as u8,
				];

				let buffer_le: [u8; 4] = [
					($instr >>  0) as u8,
					($instr >>  8) as u8,
					($instr >> 16) as u8,
					($instr >> 24) as u8,
				];

				let iisa_be = translator_be.decode(0, &buffer_be).unwrap();
				let iisa_le = translator_le.decode(0, &buffer_le).unwrap();
				assert_eq!(iisa_be, vec!(Instr{op: $translated, pred: Pred::None, exc: 0, size: 4}));
				assert_eq!(iisa_le, vec!(Instr{op: $translated, pred: Pred::None, exc: 0, size: 4}));
			}
		);
	}

	macro_rules! test_vec_r2000 {
		($func_name:ident, $pc:expr, $instrs:expr, $translated:expr) => (
			#[test]
			#[allow(non_snake_case)]
			fn $func_name() {
				let translator_be = MipsTranslator{ arch: Arch::R2000, big_endian: true };

				let mut buffer_be = Vec::new();

				for instr in $instrs.iter() {
					buffer_be.push( (instr >> 24) as u8 );
					buffer_be.push( (instr >> 16) as u8 );
					buffer_be.push( (instr >>  8) as u8 );
					buffer_be.push( (instr >>  0) as u8 );
				}

				let iisa_be = translator_be.decode($pc, &buffer_be).unwrap();
				assert_eq!(iisa_be, $translated);
			}
		);
	}

	test_simple_r2000!( r2000_addiu__gp___gp_neg12272, 0x279cd010u32, Op::Add(DstSrcSrc{dst: R::W(28), src: [Src::Reg(R::W(28)), Src::ImmI16(-12272)]}) );
	test_simple_r2000!( r2000_addiu__a0___s0_neg14244, 0x2604c85cu32, Op::Add(DstSrcSrc{dst: R::W(4),  src: [Src::Reg(R::W(16)), Src::ImmI16(-14244)]}) );

	test_simple_r2000!( r2000_addu___s2___s1_v1,       0x02239021u32, Op::Add(DstSrcSrc{dst: R::W(18), src: [Src::Reg(R::W(17)), Src::Reg(R::W(3))]}) );
	test_simple_r2000!( r2000_addu___s1___a1_zero,     0x00a08821u32, Op::Add(DstSrcSrc{dst: R::W(17), src: [Src::Reg(R::W( 5)), Src::ImmU32(0)   ]}) ); 

	test_simple_r2000!( r2000_lui____zero_0xabcd,      0x3c00abcdu32, Op::Ld(DstSrc{dst: R::Discard, src: Src::ImmU32(0xABCD0000)}) );
	test_simple_r2000!( r2000_lui____gp___0x8072,      0x3c1c8072u32, Op::Ld(DstSrc{dst: R::W(28),   src: Src::ImmU32(0x80720000)}) );

	test_simple_r2000!( r2000_ori____gp___gp_0x4354,   0x34214354u32, Op::Or(DstSrcSrc{dst: R::W(1), src: [Src::Reg(R::W(1)), Src::ImmU16(0x4354)]}) );
	test_simple_r2000!( r2000_ori____v0___v0_0xbabe,   0x3442babeu32, Op::Or(DstSrcSrc{dst: R::W(2), src: [Src::Reg(R::W(2)), Src::ImmU16(0xBABE)]}) );

	test_simple_r2000!( r2000_sw_____zero_20_____sp,   0xafa00014u32, Op::Sw(SrcSrcSrc{src: [Src::ImmU32(0),     Src::ImmI16(  20), Src::Reg(R::W(29))]}) );
	test_simple_r2000!( r2000_sw_____s3___neg336_gp,   0xaf93feb0u32, Op::Sw(SrcSrcSrc{src: [Src::Reg(R::W(19)), Src::ImmI16(-336), Src::Reg(R::W(28))]}) );

	test_vec_r2000!( r2000_beq_a2_at_80710038_move_s3_a3,
	                 0x80710028,
	                 [0x10c10003u32, 0x00e09821u32],
	                 [Instr{op: Op::Add(DstSrcSrc{dst: R::W(19), src: [Src::Reg(R::W(7)), Src::ImmU32(0)]}),                               pred: Pred::None, exc: 1, size: 0},
	                  Instr{op: Op::B(Cond::Eq, SrcSrcTarget{src: [Src::Reg(R::W(6)), Src::Reg(R::W(1))], target: Src::Addr(0x80710038)}), pred: Pred::None, exc: 2, size: 8},] );
}

