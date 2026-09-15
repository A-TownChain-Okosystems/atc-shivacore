//! TPM 2.0 ACPI discovery and CRB transport boundary.
//!
//! The TPM2 ACPI table supplies the physical CRB control-area address; ShivaCore's
//! platform layer owns the actual physical-to-virtual MMIO mapping. This module does
//! not contain key material and does not provide a software TPM fallback.

#![allow(dead_code)]

pub const TPM2_SIGNATURE: [u8; 4] = *b"TPM2";
pub const TPM2_TABLE_REVISION_MIN: u8 = 3;
pub const TPM2_TABLE_REVISION_MAX: u8 = 5;
pub const TPM2_MIN_TABLE_LEN: usize = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmStartMethod { Crb = 7, Tis = 6 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmInterface { Crb }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tpm2Descriptor { pub interface: TpmInterface, pub control_area: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmTransportError {
    InvalidDescriptor, InvalidAcpiTable, InvalidAcpiChecksum, UnsupportedRevision,
    UnsupportedStartMethod, UnsupportedInterface, AddressOverflow, BufferTooLarge,
    Timeout, DeviceError,
}

impl Tpm2Descriptor {
    pub const fn crb(control_area: u64) -> Result<Self, TpmTransportError> {
        if control_area == 0 { return Err(TpmTransportError::InvalidDescriptor); }
        Ok(Self { interface: TpmInterface::Crb, control_area })
    }

    /// Parse the ACPI TPM2 table revisions currently used by PC firmware.
    /// Revisions 3, 4 and 5 share the fields needed here: control area at byte 40
    /// and start method at byte 48. Revision 3 uses bytes 36..40 as reserved;
    /// revisions 4/5 use platform class plus reserved there.
    pub fn from_acpi(table: &[u8]) -> Result<Self, TpmTransportError> {
        if table.len() < TPM2_MIN_TABLE_LEN || table[0..4] != TPM2_SIGNATURE {
            return Err(TpmTransportError::InvalidAcpiTable);
        }
        let revision = table[8];
        if !(TPM2_TABLE_REVISION_MIN..=TPM2_TABLE_REVISION_MAX).contains(&revision) {
            return Err(TpmTransportError::UnsupportedRevision);
        }
        let declared_len = u32::from_le_bytes([table[4], table[5], table[6], table[7]]) as usize;
        if declared_len < TPM2_MIN_TABLE_LEN || declared_len > table.len() {
            return Err(TpmTransportError::InvalidAcpiTable);
        }
        if table[..declared_len].iter().fold(0u8, |s, b| s.wrapping_add(*b)) != 0 {
            return Err(TpmTransportError::InvalidAcpiChecksum);
        }
        if revision >= 4 && u16::from_le_bytes([table[38], table[39]]) != 0 {
            return Err(TpmTransportError::InvalidAcpiTable);
        }
        if revision == 3 && table[36..40].iter().any(|b| *b != 0) {
            return Err(TpmTransportError::InvalidAcpiTable);
        }
        let control_area = u64::from_le_bytes([
            table[40], table[41], table[42], table[43],
            table[44], table[45], table[46], table[47],
        ]);
        let start_method = u32::from_le_bytes([table[48], table[49], table[50], table[51]]);
        if start_method != TpmStartMethod::Crb as u32 {
            return Err(TpmTransportError::UnsupportedStartMethod);
        }
        Self::crb(control_area)
    }
}

pub trait MmioAccess {
    fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError>;
    fn write_u32(&mut self, address: u64, value: u32) -> Result<(), TpmTransportError>;
    fn read_bytes(&self, address: u64, out: &mut [u8]) -> Result<(), TpmTransportError>;
    fn write_bytes(&mut self, address: u64, data: &[u8]) -> Result<(), TpmTransportError>;
}

/// PC Client CRB register offsets relative to the Locality 0 base.
pub mod crb {
    pub const LOC_STATE: u64 = 0x00;
    pub const LOC_CTRL: u64 = 0x08;
    pub const LOC_STS: u64 = 0x0C;
    pub const CTRL_REQ: u64 = 0x40;
    pub const CTRL_STS: u64 = 0x44;
    pub const CTRL_CANCEL: u64 = 0x48;
    pub const CTRL_START: u64 = 0x4C;
    pub const INT_ENABLE: u64 = 0x50;
    pub const INT_STATUS: u64 = 0x54;
    pub const CMD_SIZE: u64 = 0x58;
    pub const CMD_ADDR_LOW: u64 = 0x5C;
    pub const CMD_ADDR_HIGH: u64 = 0x60;
    pub const RSP_SIZE: u64 = 0x64;
    pub const RSP_ADDR_LOW: u64 = 0x68;
    pub const RSP_ADDR_HIGH: u64 = 0x6C;
    pub const DATA_BUFFER: u64 = 0x80;
    pub const LOCALITY_STRIDE: u64 = 0x1000;
}

pub const MAX_TPM_TRANSFER: usize = 4096;
pub struct TpmCrbTransport<M> { descriptor: Tpm2Descriptor, pub(crate) mmio: M }
impl<M: MmioAccess> TpmCrbTransport<M> {
    pub fn new(descriptor: Tpm2Descriptor, mmio: M) -> Self { Self { descriptor, mmio } }
    pub fn descriptor(&self) -> Tpm2Descriptor { self.descriptor }
    pub fn register_address(&self, offset: u64) -> Result<u64, TpmTransportError> {
        self.descriptor.control_area.checked_add(offset).ok_or(TpmTransportError::AddressOverflow)
    }
    pub fn validate_command(command: &[u8]) -> Result<(), TpmTransportError> {
        if command.is_empty() || command.len() > MAX_TPM_TRANSFER { return Err(TpmTransportError::BufferTooLarge); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn acpi_table(revision: u8, control_area: u64, start_method: u32) -> [u8; TPM2_MIN_TABLE_LEN] {
        let mut table = [0u8; TPM2_MIN_TABLE_LEN];
        table[0..4].copy_from_slice(b"TPM2");
        table[4..8].copy_from_slice(&(TPM2_MIN_TABLE_LEN as u32).to_le_bytes());
        table[8] = revision;
        table[40..48].copy_from_slice(&control_area.to_le_bytes());
        table[48..52].copy_from_slice(&start_method.to_le_bytes());
        let sum = table.iter().fold(0u8, |s, b| s.wrapping_add(*b));
        table[9] = 0u8.wrapping_sub(sum);
        table
    }
    #[test] fn parses_revision_5() { let d = Tpm2Descriptor::from_acpi(&acpi_table(5, 0xFED4_0000, 7)).unwrap(); assert_eq!(d.interface, TpmInterface::Crb); assert_eq!(d.control_area, 0xFED4_0000); }
    #[test] fn parses_revision_4() { assert!(Tpm2Descriptor::from_acpi(&acpi_table(4, 1, 7)).is_ok()); }
    #[test] fn parses_revision_3() { assert!(Tpm2Descriptor::from_acpi(&acpi_table(3, 1, 7)).is_ok()); }
    #[test] fn rejects_revision_6() { assert_eq!(Tpm2Descriptor::from_acpi(&acpi_table(6, 1, 7)), Err(TpmTransportError::UnsupportedRevision)); }
    #[test] fn rejects_bad_signature() { let mut t = acpi_table(5, 1, 7); t[0] = b'X'; assert_eq!(Tpm2Descriptor::from_acpi(&t), Err(TpmTransportError::InvalidAcpiTable)); }
    #[test] fn rejects_bad_checksum() { let mut t = acpi_table(5, 1, 7); t[20] ^= 1; assert_eq!(Tpm2Descriptor::from_acpi(&t), Err(TpmTransportError::InvalidAcpiChecksum)); }
    #[test] fn rejects_non_crb_start_method() { assert_eq!(Tpm2Descriptor::from_acpi(&acpi_table(5, 1, 6)), Err(TpmTransportError::UnsupportedStartMethod)); }
    #[test] fn rejects_zero_control_area() { assert_eq!(Tpm2Descriptor::from_acpi(&acpi_table(5, 0, 7)), Err(TpmTransportError::InvalidDescriptor)); }
    #[test] fn rejects_bad_length() { let mut t = acpi_table(5, 1, 7); t[4..8].copy_from_slice(&51u32.to_le_bytes()); assert_eq!(Tpm2Descriptor::from_acpi(&t), Err(TpmTransportError::InvalidAcpiTable)); }
    #[test] fn validates_register_address_overflow() {
        struct NullMmio;
        impl MmioAccess for NullMmio {
            fn read_u32(&self, _: u64) -> Result<u32, TpmTransportError> { Ok(0) }
            fn write_u32(&mut self, _: u64, _: u32) -> Result<(), TpmTransportError> { Ok(()) }
            fn read_bytes(&self, _: u64, _: &mut [u8]) -> Result<(), TpmTransportError> { Ok(()) }
            fn write_bytes(&mut self, _: u64, _: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
        }
        let d = Tpm2Descriptor::crb(u64::MAX).unwrap();
        let t = TpmCrbTransport::new(d, NullMmio);
        assert_eq!(t.register_address(1), Err(TpmTransportError::AddressOverflow));
    }
    #[test] fn validates_command_bounds() {
        struct NullMmio;
        impl MmioAccess for NullMmio {
            fn read_u32(&self, _: u64) -> Result<u32, TpmTransportError> { Ok(0) }
            fn write_u32(&mut self, _: u64, _: u32) -> Result<(), TpmTransportError> { Ok(()) }
            fn read_bytes(&self, _: u64, _: &mut [u8]) -> Result<(), TpmTransportError> { Ok(()) }
            fn write_bytes(&mut self, _: u64, _: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
        }
        assert_eq!(TpmCrbTransport::<NullMmio>::validate_command(&[]), Err(TpmTransportError::BufferTooLarge));
        assert!(TpmCrbTransport::<NullMmio>::validate_command(&[0u8; 16]).is_ok());
        assert_eq!(TpmCrbTransport::<NullMmio>::validate_command(&[0u8; MAX_TPM_TRANSFER + 1]), Err(TpmTransportError::BufferTooLarge));
    }
}
