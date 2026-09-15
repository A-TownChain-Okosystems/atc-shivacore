// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! Minimal ACPI table discovery used by platform drivers.

#![cfg(feature = "x86-boot")]

use x86_64::VirtAddr;

const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";
const XSDT_SIGNATURE: &[u8; 4] = b"XSDT";
const RSDT_SIGNATURE: &[u8; 4] = b"RSDT";
const TPM2_SIGNATURE: &[u8; 4] = b"TPM2";
const RSDP_V1_LEN: usize = 20;
const RSDP_V2_LEN: usize = 36;
const SDT_HEADER_LEN: usize = 36;
const MAX_ACPI_TABLE_LEN: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    InvalidRsdp,
    InvalidChecksum,
    UnsupportedRsdp,
    InvalidTable,
    AddressOverflow,
    TableTooLarge,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiTableRef {
    pub physical_address: u64,
    pub length: usize,
}

pub struct PhysicalReader { offset: VirtAddr }

impl PhysicalReader {
    pub const fn new(offset: VirtAddr) -> Self { Self { offset } }

    fn ptr(&self, physical: u64, length: usize) -> Result<*const u8, AcpiError> {
        physical.checked_add(length as u64).ok_or(AcpiError::AddressOverflow)?;
        let virtual_address = self.offset.checked_add(physical).ok_or(AcpiError::AddressOverflow)?;
        Ok(virtual_address.as_ptr())
    }

    pub unsafe fn read(&self, physical: u64, out: &mut [u8]) -> Result<(), AcpiError> {
        let ptr = self.ptr(physical, out.len())?;
        for (index, byte) in out.iter_mut().enumerate() { *byte = core::ptr::read_volatile(ptr.add(index)); }
        Ok(())
    }

    fn read_u32(&self, physical: u64) -> Result<u32, AcpiError> {
        let mut bytes = [0u8; 4];
        unsafe { self.read(physical, &mut bytes)?; }
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&self, physical: u64) -> Result<u64, AcpiError> {
        let mut bytes = [0u8; 8];
        unsafe { self.read(physical, &mut bytes)?; }
        Ok(u64::from_le_bytes(bytes))
    }

    fn checksum(&self, physical: u64, length: usize) -> Result<(), AcpiError> {
        let mut sum = 0u8;
        let mut chunk = [0u8; 256];
        let mut offset = 0usize;
        while offset < length {
            let take = core::cmp::min(chunk.len(), length - offset);
            unsafe { self.read(physical + offset as u64, &mut chunk[..take])?; }
            for byte in &chunk[..take] { sum = sum.wrapping_add(*byte); }
            offset += take;
        }
        if sum != 0 { return Err(AcpiError::InvalidChecksum); }
        Ok(())
    }

    fn read_table_header(&self, physical: u64) -> Result<([u8; 4], usize), AcpiError> {
        let mut header = [0u8; SDT_HEADER_LEN];
        unsafe { self.read(physical, &mut header)?; }
        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        if length < SDT_HEADER_LEN { return Err(AcpiError::InvalidTable); }
        if length > MAX_ACPI_TABLE_LEN { return Err(AcpiError::TableTooLarge); }
        Ok(([header[0], header[1], header[2], header[3]], length))
    }

    pub fn root_table(&self, rsdp_physical: u64) -> Result<u64, AcpiError> {
        let mut rsdp = [0u8; RSDP_V2_LEN];
        unsafe { self.read(rsdp_physical, &mut rsdp)?; }
        if &rsdp[0..8] != RSDP_SIGNATURE { return Err(AcpiError::InvalidRsdp); }
        if rsdp[15] < 2 {
            if rsdp[..RSDP_V1_LEN].iter().fold(0u8, |s, b| s.wrapping_add(*b)) != 0 { return Err(AcpiError::InvalidChecksum); }
            return Ok(u32::from_le_bytes([rsdp[16], rsdp[17], rsdp[18], rsdp[19]]) as u64);
        }

        let length = u32::from_le_bytes([rsdp[20], rsdp[21], rsdp[22], rsdp[23]]) as usize;
        if length < RSDP_V2_LEN || length > MAX_ACPI_TABLE_LEN { return Err(AcpiError::InvalidRsdp); }
        self.checksum(rsdp_physical, length)?;
        let xsdt = u64::from_le_bytes([rsdp[24], rsdp[25], rsdp[26], rsdp[27], rsdp[28], rsdp[29], rsdp[30], rsdp[31]]);
        if xsdt != 0 { Ok(xsdt) } else { Ok(u32::from_le_bytes([rsdp[16], rsdp[17], rsdp[18], rsdp[19]]) as u64) }
    }

    pub fn find_table(&self, rsdp_physical: u64, wanted: &[u8; 4]) -> Result<AcpiTableRef, AcpiError> {
        let root = self.root_table(rsdp_physical)?;
        let (signature, root_length) = self.read_table_header(root)?;
        let is_xsdt = &signature == XSDT_SIGNATURE;
        if !is_xsdt && &signature != RSDT_SIGNATURE { return Err(AcpiError::InvalidTable); }
        self.checksum(root, root_length)?;

        let entry_size = if is_xsdt { 8 } else { 4 };
        if (root_length - SDT_HEADER_LEN) % entry_size != 0 { return Err(AcpiError::InvalidTable); }
        let count = (root_length - SDT_HEADER_LEN) / entry_size;
        for index in 0..count {
            let entry = root.checked_add((SDT_HEADER_LEN + index * entry_size) as u64).ok_or(AcpiError::AddressOverflow)?;
            let address = if is_xsdt { self.read_u64(entry)? } else { self.read_u32(entry)? as u64 };
            if address == 0 { continue; }
            let (table_signature, length) = self.read_table_header(address)?;
            if &table_signature == wanted {
                let table = AcpiTableRef { physical_address: address, length };
                self.checksum(address, length)?;
                return Ok(table);
            }
        }
        Err(AcpiError::NotFound)
    }

    pub fn find_tpm2(&self, rsdp_physical: u64) -> Result<AcpiTableRef, AcpiError> {
        self.find_table(rsdp_physical, TPM2_SIGNATURE)
    }
}
