pub mod executor;

use super::CpuReg;
use super::Error;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum R {
	Ip,
	Discard,
	Zero,

	P(u8),
	Pred(u64),
	B(u16),
	H(u16),
	W(u16),
	X(u16),

	TP(u8),
	TPred(u64),
	TB(u16),
	TH(u16),
	TW(u16),
	TX(u16),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Src {
	Reg(R),
	ImmU8(u8),
	ImmU16(u16),
	ImmU32(u32),
	ImmU64(u64),
	ImmI8(i8),
	ImmI16(i16),
	ImmI32(i32),
	ImmI64(i64),
	Addr(u64),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DstSrcSrc {
	pub dst: R,
	pub src: [Src; 2],
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SrcSrcSrc {
	pub src: [Src; 3],
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DstSrc {
	pub dst: R,
	pub src: Src,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SrcSrcTarget {
	pub src: [Src; 2],
	pub target: Src,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cond {
	Ne,
	Eq,
	Ge,
	Gt,
	Le,
	Lt,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Op {
	Nop,

	Add(DstSrcSrc),
	Sub(DstSrcSrc),
	Div(DstSrcSrc),
	Divu(DstSrcSrc),
	Mod(DstSrcSrc),
	Modu(DstSrcSrc),

	And(DstSrcSrc),
	Or(DstSrcSrc),
	Nor(DstSrcSrc),
	Sll(DstSrcSrc),
	Sra(DstSrcSrc),
	Srl(DstSrcSrc),
	Xor(DstSrcSrc),

	Set(Cond, DstSrcSrc),

	Lb(DstSrcSrc),
	Lbs(DstSrcSrc),
	Lh(DstSrcSrc),
	Lw(DstSrcSrc),
	Sb(SrcSrcSrc),
	Sh(SrcSrcSrc),
	Sw(SrcSrcSrc),
	Ld(DstSrc),

	Call(Src),
	B(Cond, SrcSrcTarget),
	Exc,
	J(Src),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Pred {
	None,
	Pred(R),
	NotPred(R),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Instr {
	pub op: Op,
	pub pred: Pred,
	pub exc: u8,
	pub size: u8,
}

pub fn is_end_of_block(op: &Op) -> bool {
	match *op {
		Op::Call(_) => true,
		Op::B(_, _) => true,
		Op::Exc     => true,
		Op::J(_)    => true,

		_ => false,
	}
}

fn interpret_op_list(instrs: &Vec<Instr>, regs: &mut RegisterFile) -> Result<(), Error> {
	for ref instr in instrs.iter() {
		match instr.op {

			//or_w_w_immu16
			Op::Or(DstSrcSrc { dst: R::W(dst_reg), src: [Src::Reg(R::W(src_reg)), Src::ImmU16(imm)]}) => {
				let result = regs.read_u32(src_reg) | (imm as u32);
				regs.write_u32(dst_reg, result);
			},

			_ => { return Err(Error::Unimplemented(format!("Unknown iisa instruction ({:?}) @ {:#x}", instr, regs.pc))); },
		}

		regs.pc += instr.size as u64;
	}
	Ok(())
}

pub struct RegisterFile {
	bytes: [u8;4096],
	pub pc: u64,
}

impl RegisterFile {
	fn new() -> RegisterFile {
		RegisterFile {
			bytes: [0; 4096],
			pc:    0,
		}
	}

	pub fn write_u32(&mut self, reg: u16, value: u32) {
		let reg_off: usize = (reg as usize) * 4;
		self.bytes[reg_off + 0] = (value >>  0) as u8;
		self.bytes[reg_off + 1] = (value >>  8) as u8;
		self.bytes[reg_off + 2] = (value >> 16) as u8;
		self.bytes[reg_off + 3] = (value >> 24) as u8;
	}

	pub fn read_u32(&self, reg: u16) -> u32 {
		let reg_off = (reg as usize) * 4;

		((self.bytes[reg_off + 0] as u32) <<  0) |
		((self.bytes[reg_off + 1] as u32) <<  8) |
		((self.bytes[reg_off + 2] as u32) << 16) |
		((self.bytes[reg_off + 3] as u32) << 24)
	}
}

pub trait Translator {
	fn decode(&self, base: u64, buffer: &[u8]) -> Result<Vec<Instr>, Error>;
	fn virtual_to_phys(&self, registers: &RegisterFile, addr: u64) -> Option<u64>;
	fn set_reg(&mut self, registers: &mut RegisterFile, reg: CpuReg, value: u64) -> Result<(), Error>;
	fn get_reg(&self, registers: &RegisterFile, reg: CpuReg) -> Result<u64, Error>;
}

