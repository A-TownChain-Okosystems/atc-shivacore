//! Minimal TPM 2.0 engine built on the CRB transport.
//!
//! The engine intentionally exposes only capability discovery and startup at this stage.
//! Key provisioning, authorization sessions, sealing and attestation remain separate
//! security-critical layers.

#![allow(dead_code)]

use super::tpm2::{MmioAccess, TpmCrbTransport, TpmTransportError, MAX_TPM_TRANSFER};
use super::tpm2_command::{TPM_HEADER_SIZE, TpmCommandError};
use super::tpm2_commands::{build_get_capability, build_startup, decode_get_capability, GetCapabilityResponse, TpmCommandBuildError, TpmProperty, TpmResponseDecodeError};

pub const TPM_RC_INITIALIZE: u32 = 0x0000_0100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmEngineError { Transport(TpmTransportError), Command(TpmCommandError), Build(TpmCommandBuildError), Decode(TpmResponseDecodeError), BufferTooSmall }
impl From<TpmTransportError> for TpmEngineError { fn from(v: TpmTransportError) -> Self { Self::Transport(v) } }
impl From<TpmCommandError> for TpmEngineError { fn from(v: TpmCommandError) -> Self { Self::Command(v) } }
impl From<TpmCommandBuildError> for TpmEngineError { fn from(v: TpmCommandBuildError) -> Self { Self::Build(v) } }
impl From<TpmResponseDecodeError> for TpmEngineError { fn from(v: TpmResponseDecodeError) -> Self { Self::Decode(v) } }

pub struct Tpm2Engine<M> { transport: TpmCrbTransport<M>, command: [u8; MAX_TPM_TRANSFER], response: [u8; MAX_TPM_TRANSFER] }

impl<M: MmioAccess> Tpm2Engine<M> {
    pub fn new(transport: TpmCrbTransport<M>) -> Self { Self { transport, command: [0; MAX_TPM_TRANSFER], response: [0; MAX_TPM_TRANSFER] } }
    pub fn transport(&self) -> &TpmCrbTransport<M> { &self.transport }

    /// Start with TPM_SU_CLEAR. TPM_RC_INITIALIZE means the TPM is already initialized.
    pub fn startup_clear(&mut self, poll_limit: usize) -> Result<(), TpmEngineError> {
        let size = build_startup(&mut self.command)?;
        let response_size = self.transport.execute(&self.command[..size], &mut self.response, poll_limit)?;
        let code = response_code(&self.response[..response_size])?;
        if code == 0 || code == TPM_RC_INITIALIZE { Ok(()) } else { Err(TpmEngineError::Command(TpmCommandError::ResponseCode(code))) }
    }

    /// Query TPM properties through TPM2_GetCapability(TPM_CAP_TPM_PROPERTIES).
    pub fn get_properties<'a>(&mut self, property: u32, count: u32, out: &'a mut [TpmProperty], poll_limit: usize) -> Result<GetCapabilityResponse<'a>, TpmEngineError> {
        let size = build_get_capability(&mut self.command, property, count)?;
        let response_size = self.transport.execute(&self.command[..size], &mut self.response, poll_limit)?;
        Ok(decode_get_capability(&self.response[..response_size], out)?)
    }
}

fn response_code(response: &[u8]) -> Result<u32, TpmEngineError> {
    if response.len() < TPM_HEADER_SIZE || response.len() > MAX_TPM_TRANSFER { return Err(TpmEngineError::BufferTooSmall); }
    let declared = u32::from_be_bytes([response[2], response[3], response[4], response[5]]) as usize;
    if declared != response.len() { return Err(TpmEngineError::Command(TpmCommandError::InvalidSize)); }
    Ok(u32::from_be_bytes([response[6], response[7], response[8], response[9]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tpm2::{Tpm2Descriptor, TpmTransportError};
    struct FakeMmio;
    impl MmioAccess for FakeMmio {
        fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError> { match address & 0xFF { 0x00 => Ok(0x80), 0x0C => Ok(1), 0x40 => Ok(0), 0x44 => Ok(2), 0x58 => Ok(64), 0x5C => Ok(0x1000), 0x60 => Ok(0), 0x64 => Ok(10), 0x68 => Ok(0x2000), 0x6C => Ok(0), 0x4C => Ok(0), _ => Ok(0) } }
        fn write_u32(&mut self, _: u64, _: u32) -> Result<(), TpmTransportError> { Ok(()) }
        fn read_bytes(&self, _: u64, out: &mut [u8]) -> Result<(), TpmTransportError> { out[..10].copy_from_slice(&[0x80,0x01,0,0,0,10,0,0,0,0]); Ok(()) }
        fn write_bytes(&mut self, _: u64, _: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
    }
    #[test]
    fn accepts_already_initialized_startup() {
        let descriptor = Tpm2Descriptor::crb(0x1000).unwrap();
        let mut engine = Tpm2Engine::new(TpmCrbTransport::new(descriptor, FakeMmio));
        assert!(engine.startup_clear(2).is_ok());
    }
}
