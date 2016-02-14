#[derive(PartialEq)]
pub enum R {
	Ip,

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

#[derive(PartialEq)]
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

#[derive(PartialEq)]
pub struct DstSrcSrc {
	dst: R,
	src: [Src; 2],
}

#[derive(PartialEq)]
pub struct SrcSrcSrc {
	src: [Src; 3],
}

#[derive(PartialEq)]
pub struct DstSrc {
	dst: R,
	src: Src,
}

#[derive(PartialEq)]
pub struct SrcSrcTarget {
	src: [Src; 2],
	target: Src,
}

#[derive(PartialEq)]
pub enum Cond {
	Ne,
	Eq,
	Ge,
	Gt,
	Le,
	Lt,
}

#[derive(PartialEq)]
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

#[derive(PartialEq)]
pub enum Pred {
	None,
	Pred(R),
	NotPred(R),
}

#[derive(PartialEq)]
pub struct Instr {
	op: Op,
	pred: Pred,
	exc: u8,
	size: u8,
}

