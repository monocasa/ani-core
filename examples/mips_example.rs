extern crate ani_core;

const RAM_BASE: u64 = 0x10000;
const RAM_SIZE: u64 = 2 * 1024 * 1024;

fn test_mips(test_name: &str, opt: ani_core::CpuOpt, code_buffer: &[u8]) -> Result<(), ani_core::Error> {
	println!("Emulate MIPS code ({})", test_name);

	let mut system = ani_core::System::new();

	try!(system.add_ram_region(ani_core::PROT_ALL, RAM_BASE, RAM_SIZE));

	try!(system.write_range(code_buffer, RAM_BASE));

	let mut cpu = try!(ani_core::mips::build_cpu(&mut system,
	                                             opt,
	                                             ani_core::mips::Arch::R2000));

	try!(cpu.set_reg(ani_core::mips::REG_AT, 0x6789));

	try!(cpu.set_reg(ani_core::CpuReg::Pc, RAM_BASE));

	try!(cpu.add_block_hook_all(Box::new(|address, size|
		println!(">>> Tracing basic block at {:#x}, block_size = {:#x}", address, size)
	)));

	try!(cpu.add_code_hook_single(RAM_BASE, Box::new(|address, size|
		println!(">>> Tracing instruction at {:#x}, instruction size = {:#x}", address, size)
	)));

	let expected_exit_pc = RAM_BASE + (code_buffer.len() as u64);
	let end_of_code = expected_exit_pc - 1;

	let exit_reason = try!(cpu.execute_range(RAM_BASE, end_of_code));

	if exit_reason != ani_core::ExitReason::PcOutOfRange(expected_exit_pc) {
		panic!("Unexpected exit reason:  {:?}", exit_reason);
	}

	println!(">>> Emulation done. Below is the CPU context");

	println!(">>> AT = {:#x}", try!(cpu.get_reg(ani_core::mips::REG_AT)));

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

