//! x86_64 TPM CRB register-MMIO adapter.
#![cfg(feature = "x86-tpm")]

use super::tpm2::{MmioAccess, TpmTransportError};
use shivacore::mmio::MmioRegion;

pub struct X86TpmMmio { region: MmioRegion }

impl X86TpmMmio {
    pub fn new(region: MmioRegion) -> Self { Self { region } }
    pub fn region(&self) -> &MmioRegion { &self.region }
    fn offset(&self, address: u64, width: usize) -> Result<usize, TpmTransportError> {
        let base = self.region.physical_start().as_u64();
        let end = address.checked_add(width as u64).ok_or(TpmTransportError::AddressOverflow)?;
        let region_end = base.checked_add(self.region.len() as u64).ok_or(TpmTransportError::AddressOverflow)?;
        if address < base || end > region_end { return Err(TpmTransportError::AddressOverflow); }
        Ok((address - base) as usize)
    }
}

impl MmioAccess for X86TpmMmio {
    fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError> {
        let offset = self.offset(address, 4)?;
        unsafe { self.region.read_u32(offset) }.map_err(|_| TpmTransportError::DeviceError)
    }
    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), TpmTransportError> {
        let offset = self.offset(address, 4)?;
        unsafe { self.region.write_u32(offset, value) }.map_err(|_| TpmTransportError::DeviceError)
    }
}

use super::tpm2::BufferAccess;
use shivacore::tpm_buffer::TpmBufferAccess;

/// Adapter for TPM command/response physical buffers. Unlike the register adapter,
/// this path uses ShivaCore's validated physical-memory boundary.
pub struct X86TpmBuffers { access: TpmBufferAccess }
impl X86TpmBuffers {
    pub const fn new(access: TpmBufferAccess) -> Self { Self { access } }
    pub fn access(&self) -> &TpmBufferAccess { &self.access }
}
impl BufferAccess for X86TpmBuffers {
    fn read(&self, address: u64, out: &mut [u8]) -> Result<(), TpmTransportError> {
        self.access.read(address, out).map_err(|_| TpmTransportError::InvalidBufferAddress)
    }
    fn write(&mut self, address: u64, data: &[u8]) -> Result<(), TpmTransportError> {
        self.access.write(address, data).map_err(|_| TpmTransportError::InvalidBufferAddress)
    }
}
