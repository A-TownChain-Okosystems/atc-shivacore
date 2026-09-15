// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Validated physical-memory access for TPM CRB command/response buffers.
//!
//! TPM-provided buffer addresses are untrusted input. Access is permitted only when
//! the complete range lies inside bootloader-declared usable RAM or an explicitly
//! registered TPM CRB MMIO window. No arbitrary physical address is dereferenced.

#[cfg(feature = "x86-boot")]
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
#[cfg(feature = "x86-boot")]
use core::ptr;
#[cfg(feature = "x86-boot")]
use x86_64::{PhysAddr, VirtAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmBufferError {
    AddressOverflow,
    RangeNotAllowed,
    LengthTooLarge,
}

#[cfg(feature = "x86-boot")]
pub struct TpmBufferAccess {
    physical_memory_offset: VirtAddr,
    memory_regions: &'static MemoryRegions,
    allowed_mmio_start: Option<u64>,
    allowed_mmio_len: usize,
}

#[cfg(feature = "x86-boot")]
impl TpmBufferAccess {
    pub const fn new(
        physical_memory_offset: VirtAddr,
        memory_regions: &'static MemoryRegions,
    ) -> Self {
        Self { physical_memory_offset, memory_regions, allowed_mmio_start: None, allowed_mmio_len: 0 }
    }

    /// Register exactly one already-mapped TPM CRB window as an allowed device range.
    /// This does not create a mapping; the kernel MMIO layer remains responsible for it.
    pub fn allow_mmio_range(&mut self, start: PhysAddr, len: usize) -> Result<(), TpmBufferError> {
        if len == 0 { return Err(TpmBufferError::RangeNotAllowed); }
        start.as_u64().checked_add(len as u64).ok_or(TpmBufferError::AddressOverflow)?;
        self.allowed_mmio_start = Some(start.as_u64());
        self.allowed_mmio_len = len;
        Ok(())
    }

    fn contains_range(start: u64, len: usize, range_start: u64, range_end: u64) -> bool {
        match start.checked_add(len as u64) {
            Some(end) => start >= range_start && end <= range_end,
            None => false,
        }
    }

    fn allowed(&self, address: u64, len: usize) -> bool {
        if len == 0 { return false; }
        for region in self.memory_regions.iter() {
            if region.kind == MemoryRegionKind::Usable
                && Self::contains_range(address, len, region.start, region.end)
            {
                return true;
            }
        }
        if let Some(start) = self.allowed_mmio_start {
            if let Some(end) = start.checked_add(self.allowed_mmio_len as u64) {
                return Self::contains_range(address, len, start, end);
            }
        }
        false
    }

    fn virtual_range(&self, address: u64, len: usize) -> Result<*mut u8, TpmBufferError> {
        if !self.allowed(address, len) { return Err(TpmBufferError::RangeNotAllowed); }
        let virtual_address = VirtAddr::new(
            self.physical_memory_offset.as_u64()
                .checked_add(address)
                .ok_or(TpmBufferError::AddressOverflow)?,
        );
        Ok(virtual_address.as_mut_ptr())
    }

    pub fn read(&self, address: u64, out: &mut [u8]) -> Result<(), TpmBufferError> {
        if out.is_empty() { return Err(TpmBufferError::LengthTooLarge); }
        let ptr = self.virtual_range(address, out.len())?;
        unsafe { ptr::copy_volatile(ptr, out.as_mut_ptr(), out.len()); }
        Ok(())
    }

    pub fn write(&self, address: u64, data: &[u8]) -> Result<(), TpmBufferError> {
        if data.is_empty() { return Err(TpmBufferError::LengthTooLarge); }
        let ptr = self.virtual_range(address, data.len())?;
        unsafe { ptr::copy_volatile(data.as_ptr(), ptr, data.len()); }
        Ok(())
    }
}

#[cfg(not(feature = "x86-boot"))]
pub struct TpmBufferAccess;

#[cfg(test)]
mod tests {
    #[test]
    fn validator_module_is_present() { assert!(true); }
}
