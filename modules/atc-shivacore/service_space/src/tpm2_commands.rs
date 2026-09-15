//! Minimal TPM 2.0 command builders and response decoders for ShivaCore.
//!
//! This module implements the first commands needed to establish a verified
//! TPM transport boundary. It does not provision keys or expose private key
//! material.
#![allow(dead_code)]

use super::tpm2::MAX_TPM_TRANSFER;
use super::tpm2_command::{parse_response_header, TPM_HEADER_SIZE, TPM_ST_NO_SESSIONS, TpmCommandError};

pub const TPM_CC_STARTUP: u32 = 0x0000_0144;
pub const TPM_CC_GET_CAPABILITY: u32 = 0x0000_017A;
pub const TPM_SU_CLEAR: u16 = 0x0000;
pub const TPM_CAP_TPM_PROPERTIES: u32 = 0x0000_0006;
pub const TPM_PT_FIXED_FIRST: u32 = 0x0000_0100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmCommandBuildError { BufferTooSmall, BufferTooLarge }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmResponseDecodeError {
    TooSmall,
    TooLarge,
    InvalidCapability,
    InvalidPropertyList,
    Response(TpmCommandError),
}
impl From<TpmCommandError> for TpmResponseDecodeError {
    fn from(value: TpmCommandError) -> Self { Self::Response(value) }
}

fn put_u16(out: &mut [u8], offset: usize, value: u16) { out[offset..offset + 2].copy_from_slice(&value.to_be_bytes()); }
fn put_u32(out: &mut [u8], offset: usize, value: u32) { out[offset..offset + 4].copy_from_slice(&value.to_be_bytes()); }
fn be_u32(input: &[u8], offset: usize) -> u32 { u32::from_be_bytes([input[offset], input[offset + 1], input[offset + 2], input[offset + 3]]) }

fn finish_header(out: &mut [u8], command_code: u32, size: usize) {
    put_u16(out, 0, TPM_ST_NO_SESSIONS);
    put_u32(out, 2, size as u32);
    put_u32(out, 6, command_code);
}

/// Build TPM2_Startup(TPM_SU_CLEAR).
pub fn build_startup(out: &mut [u8]) -> Result<usize, TpmCommandBuildError> {
    const SIZE: usize = TPM_HEADER_SIZE + 2;
    if out.len() < SIZE { return Err(TpmCommandBuildError::BufferTooSmall); }
    if SIZE > MAX_TPM_TRANSFER { return Err(TpmCommandBuildError::BufferTooLarge); }
    finish_header(out, TPM_CC_STARTUP, SIZE);
    put_u16(out, TPM_HEADER_SIZE, TPM_SU_CLEAR);
    Ok(SIZE)
}

/// Build TPM2_GetCapability(TPM_CAP_TPM_PROPERTIES, property, count).
pub fn build_get_capability(out: &mut [u8], property: u32, property_count: u32) -> Result<usize, TpmCommandBuildError> {
    const SIZE: usize = TPM_HEADER_SIZE + 12;
    if out.len() < SIZE { return Err(TpmCommandBuildError::BufferTooSmall); }
    if SIZE > MAX_TPM_TRANSFER { return Err(TpmCommandBuildError::BufferTooLarge); }
    finish_header(out, TPM_CC_GET_CAPABILITY, SIZE);
    put_u32(out, 10, TPM_CAP_TPM_PROPERTIES);
    put_u32(out, 14, property);
    put_u32(out, 18, property_count);
    Ok(SIZE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpmProperty { pub property: u32, pub value: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCapabilityResponse<'a> { pub more_data: bool, pub properties: &'a [TpmProperty] }

/// Decode a TPM2_GetCapability response containing TPM_PT properties.
pub fn decode_get_capability<'a>(response: &[u8], properties: &'a mut [TpmProperty]) -> Result<GetCapabilityResponse<'a>, TpmResponseDecodeError> {
    if response.len() < TPM_HEADER_SIZE + 9 { return Err(TpmResponseDecodeError::TooSmall); }
    if response.len() > MAX_TPM_TRANSFER { return Err(TpmResponseDecodeError::TooLarge); }
    parse_response_header(response, MAX_TPM_TRANSFER)?;
    let mut pos = TPM_HEADER_SIZE;
    let more_data = response[pos] != 0;
    pos += 1;
    if be_u32(response, pos) != TPM_CAP_TPM_PROPERTIES { return Err(TpmResponseDecodeError::InvalidCapability); }
    pos += 4;
    let count = be_u32(response, pos) as usize;
    pos += 4;
    let payload = count.checked_mul(8).ok_or(TpmResponseDecodeError::InvalidPropertyList)?;
    if count > properties.len() || payload > response.len().saturating_sub(pos) {
        return Err(TpmResponseDecodeError::InvalidPropertyList);
    }
    for property in properties.iter_mut().take(count) {
        property.property = be_u32(response, pos);
        property.value = be_u32(response, pos + 4);
        pos += 8;
    }
    Ok(GetCapabilityResponse { more_data, properties: &properties[..count] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be_u16(input: &[u8], offset: usize) -> u16 { u16::from_be_bytes([input[offset], input[offset + 1]]) }

    #[test]
    fn builds_startup_clear() {
        let mut out = [0u8; 32];
        let size = build_startup(&mut out).unwrap();
        assert_eq!(size, 12);
        assert_eq!(be_u16(&out, 0), TPM_ST_NO_SESSIONS);
        assert_eq!(be_u32(&out, 2), 12);
        assert_eq!(be_u32(&out, 6), TPM_CC_STARTUP);
        assert_eq!(be_u16(&out, 10), TPM_SU_CLEAR);
    }

    #[test]
    fn builds_get_capability() {
        let mut out = [0u8; 32];
        let size = build_get_capability(&mut out, TPM_PT_FIXED_FIRST, 8).unwrap();
        assert_eq!(size, 22);
        assert_eq!(be_u32(&out, 6), TPM_CC_GET_CAPABILITY);
        assert_eq!(be_u32(&out, 10), TPM_CAP_TPM_PROPERTIES);
        assert_eq!(be_u32(&out, 14), TPM_PT_FIXED_FIRST);
        assert_eq!(be_u32(&out, 18), 8);
    }

    #[test]
    fn decodes_property_response() {
        let mut response = [0u8; 35];
        response[0..2].copy_from_slice(&TPM_ST_NO_SESSIONS.to_be_bytes());
        response[2..6].copy_from_slice(&(35u32).to_be_bytes());
        response[6..10].copy_from_slice(&0u32.to_be_bytes());
        response[10] = 0;
        response[11..15].copy_from_slice(&TPM_CAP_TPM_PROPERTIES.to_be_bytes());
        response[15..19].copy_from_slice(&2u32.to_be_bytes());
        response[19..23].copy_from_slice(&0x100u32.to_be_bytes());
        response[23..27].copy_from_slice(&0x1234u32.to_be_bytes());
        response[27..31].copy_from_slice(&0x101u32.to_be_bytes());
        response[31..35].copy_from_slice(&0x5678u32.to_be_bytes());
        let mut properties = [TpmProperty { property: 0, value: 0 }; 2];
        let decoded = decode_get_capability(&response, &mut properties).unwrap();
        assert!(!decoded.more_data);
        assert_eq!(decoded.properties[0].property, 0x100);
        assert_eq!(decoded.properties[0].value, 0x1234);
        assert_eq!(decoded.properties[1].property, 0x101);
        assert_eq!(decoded.properties[1].value, 0x5678);
    }
}
