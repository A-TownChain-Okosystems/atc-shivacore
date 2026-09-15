//! Minimal TPM 2.0 engine built on the CRB transport.

#![allow(dead_code)]

use super::tpm2::{BufferAccess, MmioAccess, TpmCrbTransport, TpmTransportError, MAX_TPM_TRANSFER};
use super::tpm2_command::{TPM_HEADER_SIZE, TPM_RC_INITIALIZE, TpmCommandError};
use super::tpm2_commands::{build_get_capability, build_startup, decode_get_capability, GetCapabilityResponse, TpmCommandBuildError, TpmProperty, TpmResponseDecodeError};
use super::tpm2_crb::CrbTransactionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmEngineError {
    Transport(TpmTransportError),
    Transaction(CrbTransactionError),
    Command(TpmCommandError),
    Build(TpmCommandBuildError),
    Decode(TpmResponseDecodeError),
    BufferTooSmall,
}
impl From<TpmTransportError> for TpmEngineError { fn from(v: TpmTransportError) -> Self { Self::Transport(v) } }
impl From<CrbTransactionError> for TpmEngineError { fn from(v: CrbTransactionError) -> Self { Self::Transaction(v) } }
impl From<TpmCommandError> for TpmEngineError { fn from(v: TpmCommandError) -> Self { Self::Command(v) } }
impl From<TpmCommandBuildError> for TpmEngineError { fn from(v: TpmCommandBuildError) -> Self { Self::Build(v) } }
impl From<TpmResponseDecodeError> for TpmEngineError { fn from(v: TpmResponseDecodeError) -> Self { Self::Decode(v) } }

pub struct Tpm2Engine<M, B> {
    transport: TpmCrbTransport<M, B>,
    command: [u8; MAX_TPM_TRANSFER],
    response: [u8; MAX_TPM_TRANSFER],
}

impl<M: MmioAccess, B: BufferAccess> Tpm2Engine<M, B> {
    pub fn new(transport: TpmCrbTransport<M, B>) -> Self {
        Self { transport, command: [0; MAX_TPM_TRANSFER], response: [0; MAX_TPM_TRANSFER] }
    }
    pub fn transport(&self) -> &TpmCrbTransport<M, B> { &self.transport }

    pub fn startup_clear(&mut self, poll_limit: usize) -> Result<(), TpmEngineError> {
        let size = build_startup(&mut self.command)?;
        let response_size = self.transport.execute(&self.command[..size], &mut self.response, poll_limit)?;
        let code = response_code(&self.response[..response_size])?;
        if code == 0 || code == TPM_RC_INITIALIZE { Ok(()) }
        else { Err(TpmEngineError::Command(TpmCommandError::ResponseCode(code))) }
    }

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
