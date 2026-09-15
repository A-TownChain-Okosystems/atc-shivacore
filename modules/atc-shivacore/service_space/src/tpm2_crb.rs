//! TPM 2.0 CRB transaction state machine.
//!
//! Bounded command/response transfers using the CRB control area discovered from ACPI.
//! Physical MMIO mapping remains owned by the platform HAL.

#![allow(dead_code)]

use super::tpm2::{crb, MmioAccess, Tpm2Descriptor, TpmCrbTransport, TpmTransportError, MAX_TPM_TRANSFER};
use super::tpm2_command::{parse_response_header, validate_command, TpmCommandError};

const REQ_CMD_READY: u32 = 1 << 0;
const REQ_GO_IDLE: u32 = 1 << 1;
const CANCEL: u32 = 1;
const STS_TPM_IDLE: u32 = 1 << 1;
pub const DEFAULT_POLL_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrbTransactionError { Transport(TpmTransportError), Command(TpmCommandError), ResponseTooLarge, InvalidBufferAddress, PollTimeout }
impl From<TpmTransportError> for CrbTransactionError { fn from(v: TpmTransportError) -> Self { Self::Transport(v) } }
impl From<TpmCommandError> for CrbTransactionError { fn from(v: TpmCommandError) -> Self { Self::Command(v) } }

impl<M: MmioAccess> TpmCrbTransport<M> {
    /// Execute one TPM command through CRB with bounded polling and cancellation on timeout.
    pub fn execute(&mut self, command: &[u8], response: &mut [u8], poll_limit: usize) -> Result<usize, CrbTransactionError> {
        validate_command(command, MAX_TPM_TRANSFER)?;
        if response.len() < 10 || response.len() > MAX_TPM_TRANSFER { return Err(CrbTransactionError::ResponseTooLarge); }
        if poll_limit == 0 { return Err(CrbTransactionError::PollTimeout); }

        let request = self.register_address(crb::CTRL_REQ)?;
        let status = self.register_address(crb::CTRL_STS)?;
        let cancel = self.register_address(crb::CTRL_CANCEL)?;
        let start = self.register_address(crb::CTRL_START)?;
        let cmd_size = self.register_address(crb::CMD_SIZE)?;
        let cmd_low = self.register_address(crb::CMD_ADDR_LOW)?;
        let cmd_high = self.register_address(crb::CMD_ADDR_HIGH)?;
        let rsp_size = self.register_address(crb::RSP_SIZE)?;
        let rsp_low = self.register_address(crb::RSP_ADDR_LOW)?;
        let rsp_high = self.register_address(crb::RSP_ADDR_HIGH)?;

        self.mmio.write_u32(request, REQ_CMD_READY)?;
        if !self.poll_until(poll_limit, |mmio| Ok(mmio.read_u32(request)? & REQ_CMD_READY == 0))? {
            return Err(CrbTransactionError::PollTimeout);
        }

        let command_buffer_size = self.mmio.read_u32(cmd_size)? as usize;
        if command_buffer_size < command.len() || command_buffer_size > MAX_TPM_TRANSFER {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::ResponseTooLarge);
        }
        let command_address = (self.mmio.read_u32(cmd_low)? as u64) | ((self.mmio.read_u32(cmd_high)? as u64) << 32);
        if command_address == 0 {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::InvalidBufferAddress);
        }

        self.mmio.write_bytes(command_address, command)?;
        self.mmio.write_u32(start, 1)?;
        if !self.poll_until(poll_limit, |mmio| Ok(mmio.read_u32(start)? == 0))? {
            let _ = self.mmio.write_u32(cancel, CANCEL);
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::PollTimeout);
        }

        let response_address = (self.mmio.read_u32(rsp_low)? as u64) | ((self.mmio.read_u32(rsp_high)? as u64) << 32);
        let available = self.mmio.read_u32(rsp_size)? as usize;
        if response_address == 0 {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::InvalidBufferAddress);
        }
        if available < 10 || available > MAX_TPM_TRANSFER || available > response.len() {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::ResponseTooLarge);
        }

        self.mmio.read_bytes(response_address, &mut response[..available])?;
        parse_response_header(&response[..available], response.len())?;
        self.mmio.write_u32(request, REQ_GO_IDLE)?;
        if !self.poll_until(poll_limit, |mmio| Ok(mmio.read_u32(status)? & STS_TPM_IDLE != 0))? {
            return Err(CrbTransactionError::PollTimeout);
        }
        Ok(available)
    }

    fn poll_until<F>(&mut self, limit: usize, mut condition: F) -> Result<bool, CrbTransactionError>
    where F: FnMut(&M) -> Result<bool, TpmTransportError> {
        for _ in 0..limit { if condition(&self.mmio)? { return Ok(true); } }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FakeMmio;
    impl MmioAccess for FakeMmio {
        fn read_u32(&self, _: u64) -> Result<u32, TpmTransportError> { Ok(0) }
        fn write_u32(&mut self, _: u64, _: u32) -> Result<(), TpmTransportError> { Ok(()) }
        fn read_bytes(&self, _: u64, out: &mut [u8]) -> Result<(), TpmTransportError> { out.fill(0); Ok(()) }
        fn write_bytes(&mut self, _: u64, _: &[u8]) -> Result<(), TpmTransportError> { Ok(()) }
    }
    #[test]
    fn zero_poll_limit_fails_closed() {
        let descriptor = Tpm2Descriptor::crb(0x1000).unwrap();
        let mut transport = TpmCrbTransport::new(descriptor, FakeMmio);
        let command = [0x80, 0x01, 0, 0, 0, 10, 0, 0, 0, 0];
        let mut response = [0u8; 32];
        assert_eq!(transport.execute(&command, &mut response, 0), Err(CrbTransactionError::PollTimeout));
    }
}
