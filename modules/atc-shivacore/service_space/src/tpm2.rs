//! TPM 2.0 platform-discovery and CRB transport boundary.
//!
//! This module intentionally stops at the transport/HAL boundary. It does not assume
//! fixed MMIO addresses: the TPM2 ACPI table supplies the CRB control-area address.
//! Platform firmware must expose the TPM through the standardized ACPI/CRB path.

#![allow(dead_code)]

/// ACPI TPM2 table identifier (`"TPM2"`).
pub const TPM2_SIGNATURE: [u8; 4] = *b"TPM2";

/// Supported TPM 2.0 interface selected from the ACPI/platform description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmInterface {
    Crb,
}

/// Validated TPM2 platform descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tpm2Descriptor {
    pub interface: TpmInterface,
    pub control_area: u64,
}

/// TPM transport failures. No key material is represented by these errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmTransportError {
    InvalidDescriptor,
    UnsupportedInterface,
    AddressOverflow,
    BufferTooLarge,
    Timeout,
    DeviceError,
}

impl Tpm2Descriptor {
    /// Build a descriptor after validating the ACPI-provided control-area address.
    pub fn crb(control_area: u64) -> Result<Self, TpmTransportError> {
        if control_area == 0 {
            return Err(TpmTransportError::InvalidDescriptor);
        }
        Ok(Self {
            interface: TpmInterface::Crb,
            control_area,
        })
    }
}

/// Minimal MMIO access boundary used by the CRB transport.
///
/// A platform implementation owns the actual address-space mapping and must perform
/// the required memory-safety checks before exposing this interface to the service.
pub trait MmioAccess {
    fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError>;
    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), TpmTransportError>;
    fn read_bytes(&self, address: u64, out: &mut [u8]) -> Result<(), TpmTransportError>;
    fn write_bytes(&mut self, address: u64, data: &[u8]) -> Result<(), TpmTransportError>;
}

/// CRB register offsets used by the transport state machine.
pub mod crb {
    pub const LOC_STATE: u64 = 0x0000;
    pub const CTRL_REQ: u64 = 0x0040;
    pub const CTRL_STS: u64 = 0x0044;
    pub const CTRL_CANCEL: u64 = 0x0080;
    pub const CTRL_START: u64 = 0x008C;
    pub const CMD_SIZE: u64 = 0x0094;
    pub const CMD_ADDR: u64 = 0x0098;
    pub const RSP_SIZE: u64 = 0x009C;
    pub const RSP_ADDR: u64 = 0x00A0;
}

/// Maximum command/response transfer accepted by this first transport layer.
/// The limit is deliberately conservative; a future PTP revision may require larger
/// CRB transfers and should update this constant together with the buffer validation.
pub const MAX_TPM_TRANSFER: usize = 4096;

/// CRB transport wrapper. Command execution is deliberately not implemented until
/// the platform MMIO mapper and ACPI TPM2 table parser are connected.
pub struct TpmCrbTransport<M> {
    descriptor: Tpm2Descriptor,
    mmio: M,
}

impl<M: MmioAccess> TpmCrbTransport<M> {
    pub fn new(descriptor: Tpm2Descriptor, mmio: M) -> Self {
        Self { descriptor, mmio }
    }

    pub fn descriptor(&self) -> Tpm2Descriptor {
        self.descriptor
    }

    /// Validate a TPM command before it can reach MMIO.
    pub fn validate_command(command: &[u8]) -> Result<(), TpmTransportError> {
        if command.is_empty() || command.len() > MAX_TPM_TRANSFER {
            return Err(TpmTransportError::BufferTooLarge);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NullMmio;

    impl MmioAccess for NullMmio {
        fn read_u32(&self, _address: u64) -> Result<u32, TpmTransportError> { Ok(0) }
        fn write_u32(&mut self, _address: u64, _value: u32) -> Result<(), TpmTransportError> { Ok(()) }
        fn read_bytes(&self, _address: u64, out: &mut [u8]) -> Result<(), TpmTransportError> { out.fill(0); Ok(()) }
        fn write_bytes(&mut self, _address: u64, _data: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
    }

    #[test]
    fn rejects_missing_crb_address() {
        assert_eq!(Tpm2Descriptor::crb(0), Err(TpmTransportError::InvalidDescriptor));
    }

    #[test]
    fn accepts_nonzero_crb_address() {
        let descriptor = Tpm2Descriptor::crb(0xFED4_0000).unwrap();
        assert_eq!(descriptor.interface, TpmInterface::Crb);
    }

    #[test]
    fn validates_command_bounds() {
        assert_eq!(TpmCrbTransport::<NullMmio>::validate_command(&[]), Err(TpmTransportError::BufferTooLarge));
        assert!(TpmCrbTransport::<NullMmio>::validate_command(&[0u8; 16]).is_ok());
        assert_eq!(TpmCrbTransport::<NullMmio>::validate_command(&[0u8; MAX_TPM_TRANSFER + 1]), Err(TpmTransportError::BufferTooLarge));
    }
}
