//! Minimal TPM 2.0 command/response framing for the CRB transport.
//!
//! The TPM wire protocol is big-endian. CRB MMIO control registers remain
//! platform/native little-endian; this module only handles the opaque TPM wire buffer.

#![allow(dead_code)]

pub const TPM_HEADER_SIZE: usize = 10;
pub const TPM_ST_NO_SESSIONS: u16 = 0x8001;
pub const TPM_ST_SESSIONS: u16 = 0x8002;
pub const TPM_RC_SUCCESS: u32 = 0x0000_0000;
pub const TPM_RC_INITIALIZE: u32 = 0x0000_0100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmCommandError {
    TooSmall,
    TooLarge,
    InvalidTag,
    InvalidSize,
    ResponseCode(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpmResponseHeader {
    pub tag: u16,
    pub size: u32,
    pub code: u32,
}

fn be_u16(input: &[u8]) -> u16 { u16::from_be_bytes([input[0], input[1]]) }
fn be_u32(input: &[u8]) -> u32 { u32::from_be_bytes([input[0], input[1], input[2], input[3]]) }

fn valid_tag(tag: u16) -> bool { tag == TPM_ST_NO_SESSIONS || tag == TPM_ST_SESSIONS }

pub fn validate_command(command: &[u8], max_size: usize) -> Result<(), TpmCommandError> {
    if command.len() < TPM_HEADER_SIZE { return Err(TpmCommandError::TooSmall); }
    if command.len() > max_size { return Err(TpmCommandError::TooLarge); }
    let tag = be_u16(&command[0..2]);
    if !valid_tag(tag) { return Err(TpmCommandError::InvalidTag); }
    let size = be_u32(&command[2..6]) as usize;
    if size != command.len() { return Err(TpmCommandError::InvalidSize); }
    Ok(())
}

pub fn parse_response_header(response: &[u8], max_size: usize) -> Result<TpmResponseHeader, TpmCommandError> {
    if response.len() < TPM_HEADER_SIZE { return Err(TpmCommandError::TooSmall); }
    if response.len() > max_size { return Err(TpmCommandError::TooLarge); }
    let header = TpmResponseHeader {
        tag: be_u16(&response[0..2]),
        size: be_u32(&response[2..6]),
        code: be_u32(&response[6..10]),
    };
    if !valid_tag(header.tag) { return Err(TpmCommandError::InvalidTag); }
    if header.size as usize < TPM_HEADER_SIZE || header.size as usize > max_size {
        return Err(TpmCommandError::InvalidSize);
    }
    if header.size as usize != response.len() { return Err(TpmCommandError::InvalidSize); }
    if header.code != TPM_RC_SUCCESS { return Err(TpmCommandError::ResponseCode(header.code)); }
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(tag: u16, size: u32) -> [u8; TPM_HEADER_SIZE] {
        let mut value = [0u8; TPM_HEADER_SIZE];
        value[0..2].copy_from_slice(&tag.to_be_bytes());
        value[2..6].copy_from_slice(&size.to_be_bytes());
        value
    }

    #[test]
    fn accepts_valid_command_header() {
        let value = command(TPM_ST_NO_SESSIONS, TPM_HEADER_SIZE as u32);
        assert!(validate_command(&value, 4096).is_ok());
    }

    #[test]
    fn accepts_session_command_header() {
        let value = command(TPM_ST_SESSIONS, TPM_HEADER_SIZE as u32);
        assert!(validate_command(&value, 4096).is_ok());
    }

    #[test]
    fn rejects_wrong_wire_endian_tag() {
        let value = command(TPM_ST_NO_SESSIONS.swap_bytes(), TPM_HEADER_SIZE as u32);
        assert_eq!(validate_command(&value, 4096), Err(TpmCommandError::InvalidTag));
    }

    #[test]
    fn rejects_declared_size_mismatch() {
        let value = command(TPM_ST_NO_SESSIONS, (TPM_HEADER_SIZE + 1) as u32);
        assert_eq!(validate_command(&value, 4096), Err(TpmCommandError::InvalidSize));
    }

    #[test]
    fn accepts_success_response() {
        let value = command(TPM_ST_NO_SESSIONS, TPM_HEADER_SIZE as u32);
        assert_eq!(parse_response_header(&value, 4096).unwrap().code, TPM_RC_SUCCESS);
    }

    #[test]
    fn accepts_session_response() {
        let value = command(TPM_ST_SESSIONS, TPM_HEADER_SIZE as u32);
        assert_eq!(parse_response_header(&value, 4096).unwrap().tag, TPM_ST_SESSIONS);
    }

    #[test]
    fn rejects_tpm_error_response() {
        let mut value = command(TPM_ST_NO_SESSIONS, TPM_HEADER_SIZE as u32);
        value[6..10].copy_from_slice(&0x0000_0184u32.to_be_bytes());
        assert_eq!(parse_response_header(&value, 4096), Err(TpmCommandError::ResponseCode(0x0000_0184)));
    }

    #[test]
    fn accepts_initialize_response_code_for_engine_handling() {
        let mut value = command(TPM_ST_NO_SESSIONS, TPM_HEADER_SIZE as u32);
        value[6..10].copy_from_slice(&TPM_RC_INITIALIZE.to_be_bytes());
        assert_eq!(parse_response_header(&value, 4096), Err(TpmCommandError::ResponseCode(TPM_RC_INITIALIZE)));
    }
}
