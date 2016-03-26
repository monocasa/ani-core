extern crate libc;

use super::{MemProt, Error};

use std::mem;
use std::ptr;

#[allow(dead_code)]
pub enum MemRangeImpl {
	Mappable(*mut u8)
}

#[allow(dead_code)]
pub struct MemRange {
	base: u64,
	size: u64,
	prot: MemProt,
	backing: MemRangeImpl,
}

//struct 

#[derive(Default)]
pub struct MemMap {
	ranges: Vec<MemRange>,
}

impl MemMap {
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

		self.ranges.push(MemRange{base: base, size: size, prot: prot, backing: MemRangeImpl::Mappable(ptr)});

		Ok(())
	}

	fn find_range(&self, base: u64, len: usize) -> Result<*mut u8, Error> {
		let end = base + (len as u64);
		for ref range in self.ranges.iter() {
			let range_end = range.base + range.size;
			match range.backing {
				MemRangeImpl::Mappable(buffer) => {
					if (base >= range.base) && (base < range_end) &&
					   (end > range.base) && (end <= range_end) {
						let offset = base - range.base;
						unsafe {
							return Ok(buffer.offset(offset as isize));
						}
					}
				},
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

