extern crate ani_core;

const ROM_BASE: u64 = 0x1FC00000;
const ROM_SIZE: u64 = 256 * 1024;
const ROM_VIRT: u64 = ROM_BASE + 0xA0000000;

fn test_mips(test_name: &str, opt: ani_core::CpuOpt, code_buffer: &[u8]) -> Result<(), ani_core::Error> {
	println!("Emulate MIPS code ({})", test_name);

	let mut system = ani_core::System::new();

	try!(system.add_mappable_range(ani_core::PROT_ALL, ROM_BASE, ROM_SIZE));

	try!(system.set_range(code_buffer, ROM_BASE));

	let cpu = try!(system.register_cpu(opt, ani_core::Arch::Mips(ani_core::mips::Arch::R2000)));

	try!(system.set_cpu_reg(&cpu, ani_core::mips::REG_AT, 0x6789));

	try!(system.set_cpu_reg(&cpu, ani_core::CpuReg::Pc, ROM_VIRT));

	try!(system.add_block_hook_all(Box::new(|address, size|
		println!(">>> Tracing basic block at {:#x}, block_size = {:#x}", address, size)
	)));

	try!(system.add_code_hook_single(ROM_BASE, Box::new(|address, size|
		println!(">>> Tracing instruction at {:#x}, instruction size = {:#x}", address, size)
	)));

	let expected_exit_pc = ROM_VIRT + (code_buffer.len() as u64);
	let end_of_code = expected_exit_pc - 1;

	let exit_reason = try!(system.execute_cpu_range(&cpu, ROM_VIRT, end_of_code));

	if exit_reason != ani_core::ExitReason::PcOutOfRange(expected_exit_pc) {
		panic!("Unexpected exit reason:  {:?}", exit_reason);
	}

	println!(">>> Emulation done. Below is the CPU context");

	println!(">>> AT = {:#x}", try!(system.get_cpu_reg(&cpu, ani_core::mips::REG_AT)));

	Ok(())
}

fn main() {
	const MIPS_CODE_EB: [u8; 4] = [0x34, 0x21, 0x34, 0x56]; // ori $at, $at, 0x3456
	const MIPS_CODE_EL: [u8; 4] = [0x56, 0x34, 0x21, 0x34]; // ori $at, $at, 0x3456

	test_mips("big-endian",
	          ani_core::CPU_ENDIAN_BIG,
	          &MIPS_CODE_EB).unwrap();

	test_mips("little-endian",
	          ani_core::CPU_ENDIAN_LITTLE,
	          &MIPS_CODE_EL).unwrap();
}

