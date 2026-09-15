//! TPM 2.0 CRB transaction state machine.

#![allow(dead_code)]

use super::tpm2::{crb, BufferAccess, MmioAccess, TpmCrbTransport, TpmTransportError, MAX_TPM_TRANSFER};
use super::tpm2_command::{parse_response_header, validate_command, TpmCommandError};

const REQ_CMD_READY: u32 = 1 << 0;
const REQ_GO_IDLE: u32 = 1 << 1;
const CANCEL: u32 = 1;
const START: u32 = 1;
const STS_TPM_IDLE: u32 = 1 << 1;
const LOC_REQUEST_ACCESS: u32 = 1 << 0;
const LOC_RELINQUISH: u32 = 1 << 1;
const LOC_ASSIGNED: u32 = 1 << 1;
const LOC_VALID: u32 = 1 << 7;
const LOC_ACTIVE_MASK: u32 = 0x1C;
const LOC_ACTIVE_SHIFT: u32 = 2;
const LOC_GRANTED: u32 = 1 << 0;
pub const DEFAULT_POLL_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrbTransactionError {
    Transport(TpmTransportError), Command(TpmCommandError), ResponseTooLarge,
    InvalidBufferAddress, LocalityUnavailable, WrongLocality, PollTimeout,
}
impl From<TpmTransportError> for CrbTransactionError { fn from(v: TpmTransportError) -> Self { Self::Transport(v) } }
impl From<TpmCommandError> for CrbTransactionError { fn from(v: TpmCommandError) -> Self { Self::Command(v) } }

impl<M: MmioAccess, B: BufferAccess> TpmCrbTransport<M, B> {
    pub fn execute(&mut self, command: &[u8], response: &mut [u8], poll_limit: usize) -> Result<usize, CrbTransactionError> {
        validate_command(command, MAX_TPM_TRANSFER)?;
        if response.len() < 10 || response.len() > MAX_TPM_TRANSFER { return Err(CrbTransactionError::ResponseTooLarge); }
        if poll_limit == 0 { return Err(CrbTransactionError::PollTimeout); }
        self.acquire_locality(poll_limit)?;
        let result = self.execute_owned(command, response, poll_limit);
        let release = self.release_locality(poll_limit);
        match (result, release) {
            (Err(error), _) => Err(error),
            (Ok(size), Ok(())) => Ok(size),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn acquire_locality(&mut self, poll_limit: usize) -> Result<(), CrbTransactionError> {
        let state = self.register_address(crb::LOC_STATE)?;
        let control = self.register_address(crb::LOC_CTRL)?;
        let status = self.register_address(crb::LOC_STS)?;
        let initial = self.mmio.read_u32(state)?;
        if initial & LOC_VALID == 0 { return Err(CrbTransactionError::LocalityUnavailable); }
        if initial & LOC_ASSIGNED != 0 {
            if ((initial & LOC_ACTIVE_MASK) >> LOC_ACTIVE_SHIFT) != 0 { return Err(CrbTransactionError::WrongLocality); }
            if self.mmio.read_u32(status)? & LOC_GRANTED != 0 { return Ok(()); }
        }
        self.mmio.write_u32(control, LOC_REQUEST_ACCESS)?;
        for _ in 0..poll_limit {
            let value = self.mmio.read_u32(state)?;
            if value & LOC_VALID == 0 { continue; }
            if value & LOC_ASSIGNED != 0 {
                if ((value & LOC_ACTIVE_MASK) >> LOC_ACTIVE_SHIFT) != 0 { return Err(CrbTransactionError::WrongLocality); }
                if self.mmio.read_u32(status)? & LOC_GRANTED != 0 { return Ok(()); }
            }
        }
        Err(CrbTransactionError::PollTimeout)
    }

    fn release_locality(&mut self, poll_limit: usize) -> Result<(), CrbTransactionError> {
        let state = self.register_address(crb::LOC_STATE)?;
        let control = self.register_address(crb::LOC_CTRL)?;
        self.mmio.write_u32(control, LOC_RELINQUISH)?;
        for _ in 0..poll_limit {
            let value = self.mmio.read_u32(state)?;
            if value & LOC_VALID != 0 && value & LOC_ASSIGNED == 0 { return Ok(()); }
        }
        Err(CrbTransactionError::PollTimeout)
    }

    fn execute_owned(&mut self, command: &[u8], response: &mut [u8], poll_limit: usize) -> Result<usize, CrbTransactionError> {
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
        for _ in 0..poll_limit { if self.mmio.read_u32(request)? & REQ_CMD_READY == 0 { break; } }
        if self.mmio.read_u32(request)? & REQ_CMD_READY != 0 { return Err(CrbTransactionError::PollTimeout); }

        let command_buffer_size = self.mmio.read_u32(cmd_size)? as usize;
        if command_buffer_size < command.len() || command_buffer_size > MAX_TPM_TRANSFER {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE); return Err(CrbTransactionError::ResponseTooLarge);
        }
        let command_address = (self.mmio.read_u32(cmd_low)? as u64) | ((self.mmio.read_u32(cmd_high)? as u64) << 32);
        if command_address == 0 { let _ = self.mmio.write_u32(request, REQ_GO_IDLE); return Err(CrbTransactionError::InvalidBufferAddress); }
        self.buffers.write(command_address, command)?;

        self.mmio.write_u32(start, START)?;
        let mut completed = false;
        for _ in 0..poll_limit { if self.mmio.read_u32(start)? == 0 { completed = true; break; } }
        if !completed {
            let _ = self.mmio.write_u32(cancel, CANCEL);
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE);
            return Err(CrbTransactionError::PollTimeout);
        }

        let response_address = (self.mmio.read_u32(rsp_low)? as u64) | ((self.mmio.read_u32(rsp_high)? as u64) << 32);
        let available = self.mmio.read_u32(rsp_size)? as usize;
        if response_address == 0 { let _ = self.mmio.write_u32(request, REQ_GO_IDLE); return Err(CrbTransactionError::InvalidBufferAddress); }
        if available < 10 || available > MAX_TPM_TRANSFER || available > response.len() {
            let _ = self.mmio.write_u32(request, REQ_GO_IDLE); return Err(CrbTransactionError::ResponseTooLarge);
        }
        self.buffers.read(response_address, &mut response[..available])?;
        parse_response_header(&response[..available], available)?;
        self.mmio.write_u32(request, REQ_GO_IDLE)?;
        for _ in 0..poll_limit { if self.mmio.read_u32(status)? & STS_TPM_IDLE != 0 { return Ok(available); } }
        Err(CrbTransactionError::PollTimeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FakeMmio { state: u32, status: u32 }
    impl MmioAccess for FakeMmio {
        fn read_u32(&self, address: u64) -> Result<u32, TpmTransportError> {
            match address & 0xFF { 0x00 => Ok(self.state), 0x0C => Ok(self.status), 0x40 => Ok(0), 0x44 => Ok(STS_TPM_IDLE), 0x58 => Ok(64), 0x5C => Ok(0x1000), 0x60 => Ok(0), 0x64 => Ok(10), 0x68 => Ok(0x2000), 0x6C => Ok(0), 0x4C => Ok(0), _ => Ok(0) }
        }
        fn write_u32(&mut self, address: u64, value: u32) -> Result<(), TpmTransportError> {
            match address & 0xFF { 0x08 if value == LOC_REQUEST_ACCESS => { self.state = LOC_VALID | LOC_ASSIGNED; self.status = LOC_GRANTED; }, 0x08 if value == LOC_RELINQUISH => self.state = LOC_VALID, _ => {} } Ok(())
        }
    }
    struct FakeBuffers;
    impl BufferAccess for FakeBuffers {
        fn read(&self, address: u64, out: &mut [u8]) -> Result<(), TpmTransportError> {
            assert_eq!(address, 0x2000); out.fill(0); out[..2].copy_from_slice(&0x8001u16.to_be_bytes()); out[2..6].copy_from_slice(&10u32.to_be_bytes()); Ok(())
        }
        fn write(&mut self, address: u64, _: &[u8]) -> Result<(), TpmTransportError> { assert_eq!(address, 0x1000); Ok(()) }
    }
    #[test]
    fn zero_poll_limit_fails_closed() {
        let descriptor = super::super::tpm2::Tpm2Descriptor::crb(0x1000).unwrap();
        let mut transport = TpmCrbTransport::new(descriptor, FakeMmio { state: LOC_VALID, status: 0 }, FakeBuffers);
        let command = [0x80, 0x01, 0, 0, 0, 10, 0, 0, 0, 0]; let mut response = [0u8; 32];
        assert_eq!(transport.execute(&command, &mut response, 0), Err(CrbTransactionError::PollTimeout));
    }
}
