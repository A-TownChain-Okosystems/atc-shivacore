//! TPM 2.0 CRB locality-0 acquisition and release.
//!
//! Locality is mandatory before CRB control/data registers may be used. This
//! module owns only the locality protocol; physical MMIO mapping remains in
//! the platform HAL.

#![allow(dead_code)]

use super::tpm2::{crb, MmioAccess, Tpm2Descriptor, TpmTransportError};

const REQUEST_ACCESS: u32 = 1 << 0;
const RELINQUISH: u32 = 1 << 1;
const LOC_ASSIGNED: u32 = 1 << 1;
const ACTIVE_LOCALITY_MASK: u32 = 0x1C;
const ACTIVE_LOCALITY_SHIFT: u32 = 2;
const REG_VALID: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalityError {
    Transport(TpmTransportError),
    InvalidState,
    NotGranted,
    WrongLocality,
    PollTimeout,
}

impl From<TpmTransportError> for LocalityError {
    fn from(value: TpmTransportError) -> Self { Self::Transport(value) }
}

pub struct TpmLocality<M> {
    descriptor: Tpm2Descriptor,
    mmio: M,
}

impl<M: MmioAccess> TpmLocality<M> {
    pub fn new(descriptor: Tpm2Descriptor, mmio: M) -> Self {
        Self { descriptor, mmio }
    }

    fn address(&self, offset: u64) -> Result<u64, LocalityError> {
        self.descriptor.control_area.checked_add(offset).ok_or(LocalityError::Transport(TpmTransportError::AddressOverflow))
    }

    /// Acquire Locality 0 and wait until the TPM reports it as active.
    pub fn acquire(&mut self, poll_limit: usize) -> Result<(), LocalityError> {
        if poll_limit == 0 { return Err(LocalityError::PollTimeout); }
        let state = self.address(crb::LOC_STATE)?;
        let control = self.address(crb::LOC_CTRL)?;
        let status = self.address(crb::LOC_STS)?;

        let initial = self.mmio.read_u32(state)?;
        if initial & REG_VALID == 0 { return Err(LocalityError::InvalidState); }

        self.mmio.write_u32(control, REQUEST_ACCESS)?;
        for _ in 0..poll_limit {
            let value = self.mmio.read_u32(state)?;
            if value & REG_VALID == 0 { continue; }
            if value & LOC_ASSIGNED != 0 {
                let locality = (value & ACTIVE_LOCALITY_MASK) >> ACTIVE_LOCALITY_SHIFT;
                if locality != 0 { return Err(LocalityError::WrongLocality); }
                if self.mmio.read_u32(status)? & 1 != 0 { return Ok(()); }
            }
        }
        Err(LocalityError::PollTimeout)
    }

    /// Release Locality 0. The operation is best-effort because the TPM owns
    /// arbitration and may have transitioned independently after an error.
    pub fn release(&mut self) -> Result<(), LocalityError> {
        let control = self.address(crb::LOC_CTRL)?;
        self.mmio.write_u32(control, RELINQUISH)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeMmio { state: u32, status: u32, writes: usize }
    impl MmioAccess for FakeMmio {
        fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError> {
            match address & 0xFF { 0x00 => Ok(self.state), 0x0C => Ok(self.status), _ => Ok(0) }
        }
        fn write_u32(&mut self, address: u64, value: u32) -> Result<(), TpmTransportError> {
            self.writes += 1;
            if address & 0xFF == 0x08 && value == REQUEST_ACCESS { self.state = REG_VALID | LOC_ASSIGNED; self.status = 1; }
            Ok(())
        }
        fn read_bytes(&self, _: u64, _: &mut [u8]) -> Result<(), TpmTransportError> { Ok(()) }
        fn write_bytes(&mut self, _: u64, _: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
    }

    #[test]
    fn acquires_locality_zero() {
        let descriptor = Tpm2Descriptor::crb(0x1000).unwrap();
        let mut locality = TpmLocality::new(descriptor, FakeMmio { state: REG_VALID, status: 0, writes: 0 });
        assert_eq!(locality.acquire(4), Ok(()));
    }

    #[test]
    fn rejects_invalid_state() {
        let descriptor = Tpm2Descriptor::crb(0x1000).unwrap();
        let mut locality = TpmLocality::new(descriptor, FakeMmio { state: 0, status: 0, writes: 0 });
        assert_eq!(locality.acquire(1), Err(LocalityError::InvalidState));
    }

    #[test]
    fn zero_poll_limit_fails_closed() {
        let descriptor = Tpm2Descriptor::crb(0x1000).unwrap();
        let mut locality = TpmLocality::new(descriptor, FakeMmio { state: REG_VALID, status: 0, writes: 0 });
        assert_eq!(locality.acquire(0), Err(LocalityError::PollTimeout));
    }
}
