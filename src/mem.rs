extern crate libc;

use super::{MemProt, Error, PROT_READ};

use std::mem;
use std::ptr;
use std::sync;

#[derive(Debug, Eq, PartialEq)]
pub enum ReadResult<T> {
	Unaligned,
	BusError,
	Success(T),
}

#[derive(Debug, Eq, PartialEq)]
pub enum WriteResult {
	Unaligned,
	BusError,
	Success,
}

pub trait BusSlave {
	fn read_u8(&mut self, addr: u64) -> ReadResult<u8>;
	fn read_u16(&mut self, addr: u64) -> ReadResult<u16>;
	fn read_u32(&mut self, addr: u64) -> ReadResult<u32>;
	fn read_u64(&mut self, addr: u64) -> ReadResult<u64>;
	fn write_u8(&mut self, addr: u64, data: u8) -> WriteResult;
	fn write_u16(&mut self, addr: u64, data: u16) -> WriteResult;
	fn write_u32(&mut self, addr: u64, data: u32) -> WriteResult;
	fn write_u64(&mut self, addr: u64, data: u64) -> WriteResult;
}

pub enum MemRangeImpl {
	Mappable(*mut u8, MemProt),
	Mmio(sync::Mutex<Box<BusSlave>>),
}

pub struct MemRange {
	base: u64,
	size: u64,
	backing: MemRangeImpl,
}

#[derive(Default)]
pub struct BusMatrix {
	ranges: Vec<MemRange>,
}

impl BusMatrix {
	pub fn add_mappable_range(&mut self, base: u64, size: u64, prot: MemProt) -> Result<(), Error> {
		let ptr: *mut u8 = unsafe {
			let mut page_mem: *mut libc::c_void = mem::transmute(0 as usize);

			match libc::posix_memalign(&mut page_mem, 4096, size as libc::size_t) {
				0 => {
					page_mem as *mut u8
				},

				_ => {
					return Err(Error::MemAllocation);
				},
			}
		};

		self.ranges.push(MemRange{base: base, size: size, backing: MemRangeImpl::Mappable(ptr, prot)});

		Ok(())
	}

	pub fn add_bus_slave(&mut self, base: u64, size: u64, slave: sync::Mutex<Box<BusSlave>>) -> Result<(), Error> {
		self.ranges.push(MemRange{base: base, size: size, backing: MemRangeImpl::Mmio(slave)});

		Ok(())
	}

	fn find_range(&self, base: u64, len: usize) -> Result<*mut u8, Error> {
		let end = base + (len as u64);
		for ref range in self.ranges.iter() {
			let range_end = range.base + range.size;
			match range.backing {
				MemRangeImpl::Mappable(buffer, _) => {
					if (base >= range.base) && (base < range_end) &&
					   (end > range.base) && (end <= range_end) {
						let offset = base - range.base;
						unsafe {
							return Ok(buffer.offset(offset as isize));
						}
					}
				},
				_ => {},// Skip everything else
			}
		}
		Err(Error::UnableToFindRange)
	}

	pub fn set_range(&mut self, incoming: &[u8], base: u64) -> Result<(), Error> {
		let backing_range = try!(self.find_range(base, incoming.len()));

		unsafe {
			ptr::copy_nonoverlapping(incoming.as_ptr(), backing_range, incoming.len());
		}

		Ok(())
	}
}

impl BusSlave for BusMatrix {
	fn read_u8(&mut self, addr: u64) -> ReadResult<u8> {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 1) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						ReadResult::Success(unsafe {
							*(((buffer as u64) + addr_offset) as *mut u8)
						})
					} else {
						ReadResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return ReadResult::BusError;
						},
					};
						
					slave.read_u8(addr_offset)
				},
			};
		}

		ReadResult::BusError
	}

	fn read_u16(&mut self, addr: u64) -> ReadResult<u16> {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 2) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						ReadResult::Success(unsafe {
							*(((buffer as u64) + addr_offset) as *mut u16)
						})
					} else {
						ReadResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return ReadResult::BusError;
						},
					};
						
					slave.read_u16(addr_offset)
				},
			};
		}

		ReadResult::BusError
	}

	fn read_u32(&mut self, addr: u64) -> ReadResult<u32> {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 4) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						ReadResult::Success(unsafe {
							*(((buffer as u64) + addr_offset) as *mut u32)
						})
					} else {
						ReadResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return ReadResult::BusError;
						},
					};
						
					slave.read_u32(addr_offset)
				},
			};
		}

		ReadResult::BusError
	}

	fn read_u64(&mut self, addr: u64) -> ReadResult<u64> {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 8) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						ReadResult::Success(unsafe {
							*(((buffer as u64) + addr_offset) as *mut u64)
						})
					} else {
						ReadResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return ReadResult::BusError;
						},
					};
						
					slave.read_u64(addr_offset)
				},
			};
		}

		ReadResult::BusError
	}

	fn write_u8(&mut self, addr: u64, data: u8) -> WriteResult {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 1) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						unsafe {
							*(((buffer as u64) + addr_offset) as *mut u8) = data;
						}
						WriteResult::Success
					} else {
						WriteResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return WriteResult::BusError;
						},
					};
						
					slave.write_u8(addr_offset, data)
				},
			};
		}

		WriteResult::BusError
	}

	fn write_u16(&mut self, addr: u64, data: u16) -> WriteResult {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 1) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						unsafe {
							*(((buffer as u64) + addr_offset) as *mut u16) = data;
						}
						WriteResult::Success
					} else {
						WriteResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return WriteResult::BusError;
						},
					};
						
					slave.write_u16(addr_offset, data)
				},
			};
		}

		WriteResult::BusError
	}

	fn write_u32(&mut self, addr: u64, data: u32) -> WriteResult {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 1) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						unsafe {
							*(((buffer as u64) + addr_offset) as *mut u32) = data;
						}
						WriteResult::Success
					} else {
						WriteResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return WriteResult::BusError;
						},
					};
						
					slave.write_u32(addr_offset, data)
				},
			};
		}

		WriteResult::BusError
	}

	fn write_u64(&mut self, addr: u64, data: u64) -> WriteResult {
		for mut slave in self.ranges.iter_mut() {
			if (addr < slave.base) || ((addr + 1) >= (slave.base + slave.size)) {
				continue;
			}

			let addr_offset = addr - slave.base;
			return match slave.backing {
				MemRangeImpl::Mappable(buffer, prot) => {
					if prot.contains(PROT_READ) {
						unsafe {
							*(((buffer as u64) + addr_offset) as *mut u64) = data;
						}
						WriteResult::Success
					} else {
						WriteResult::BusError
					}
				},
				MemRangeImpl::Mmio(ref mut slave_mutex) => {
					let mut slave = match slave_mutex.lock() {
						Ok(slave) => slave,
						Err(_) => {
							return WriteResult::BusError;
						},
					};
						
					slave.write_u64(addr_offset, data)
				},
			};
		}

		WriteResult::BusError
	}
}

#[cfg(test)]
mod tests {
	use super::{BusMatrix, BusSlave, ReadResult, WriteResult};

	use std::sync;

	#[derive(Debug, Eq, PartialEq)]
	enum BusAccess {
		ReadU8(u64),
		ReadU16(u64),
		ReadU32(u64),
		ReadU64(u64),
		WriteU8(u64, u8),
		WriteU16(u64, u16),
		WriteU32(u64, u32),
		WriteU64(u64, u64),
	}

	struct TestBusSlave {
		pub accesses: Vec<BusAccess>,
	}

	impl TestBusSlave {
		fn new() -> TestBusSlave {
			TestBusSlave {
				accesses: Vec::new(),
			}
		}
	}

	impl BusSlave for TestBusSlave {
		fn read_u8(&mut self, addr: u64) -> ReadResult<u8> {
			self.accesses.push(BusAccess::ReadU8(addr));

			ReadResult::Success(self.accesses.len() as u8)
		}

		fn read_u16(&mut self, addr: u64) -> ReadResult<u16> {
			self.accesses.push(BusAccess::ReadU16(addr));

			ReadResult::Success(self.accesses.len() as u16)
		}

		fn read_u32(&mut self, addr: u64) -> ReadResult<u32> {
			self.accesses.push(BusAccess::ReadU32(addr));

			ReadResult::Success(self.accesses.len() as u32)
		}

		fn read_u64(&mut self, addr: u64) -> ReadResult<u64> {
			self.accesses.push(BusAccess::ReadU64(addr));

			ReadResult::Success(self.accesses.len() as u64)
		}

		fn write_u8(&mut self, addr: u64, data: u8) -> WriteResult {
			self.accesses.push(BusAccess::WriteU8(addr, data));

			WriteResult::Success
		}

		fn write_u16(&mut self, addr: u64, data: u16) -> WriteResult {
			self.accesses.push(BusAccess::WriteU16(addr, data));

			WriteResult::Success
		}

		fn write_u32(&mut self, addr: u64, data: u32) -> WriteResult {
			self.accesses.push(BusAccess::WriteU32(addr, data));

			WriteResult::Success
		}

		fn write_u64(&mut self, addr: u64, data: u64) -> WriteResult {
			self.accesses.push(BusAccess::WriteU64(addr, data));

			WriteResult::Success
		}
	}

	#[test]
	fn simple_dispatch() {
		let mut map: BusMatrix = Default::default();

		let boxed_slave: Box<BusSlave> = Box::new(TestBusSlave::new());

		let bus_slave = sync::Mutex::new(boxed_slave);

		map.add_bus_slave(0x1000, 0x200, bus_slave).unwrap();

		assert_eq!(ReadResult::Success(1), map.read_u8(0x1000));
		assert_eq!(ReadResult::Success(2), map.read_u8(0x1000));
		assert_eq!(ReadResult::BusError, map.read_u8(0x1200));
	}
}

