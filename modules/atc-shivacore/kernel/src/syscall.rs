// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//! ShivaCore syscall ABI v1 foundation.

#![allow(dead_code)]

pub const ABI_VERSION: u16 = 1;
pub const MAX_ARGS: usize = 6;

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    ProcessSelf = 0x0001,
    ProcessExit = 0x0002,
    ThreadYield = 0x0101,
    ThreadExit = 0x0102,
    MemoryMap = 0x0201,
    MemoryUnmap = 0x0202,
    IpcSend = 0x0301,
    IpcReceive = 0x0302,
    CapabilityGrant = 0x0401,
    CapabilityRevoke = 0x0402,
    SystemGetTime = 0x0501,
    SystemInfo = 0x0502,
}

impl Syscall {
    pub const fn from_raw(value: u64) -> Option<Self> {
        match value {
            0x0001 => Some(Self::ProcessSelf),
            0x0002 => Some(Self::ProcessExit),
            0x0101 => Some(Self::ThreadYield),
            0x0102 => Some(Self::ThreadExit),
            0x0201 => Some(Self::MemoryMap),
            0x0202 => Some(Self::MemoryUnmap),
            0x0301 => Some(Self::IpcSend),
            0x0302 => Some(Self::IpcReceive),
            0x0401 => Some(Self::CapabilityGrant),
            0x0402 => Some(Self::CapabilityRevoke),
            0x0501 => Some(Self::SystemGetTime),
            0x0502 => Some(Self::SystemInfo),
            _ => None,
        }
    }
}

#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Success = 0,
    InvalidSyscall = -1,
    InvalidArgument = -2,
    InvalidPointer = -3,
    PermissionDenied = -4,
    CapabilityNotFound = -5,
    ResourceNotFound = -6,
    ResourceExhausted = -7,
    WouldBlock = -8,
    NotSupported = -9,
    InvalidState = -10,
    Internal = -11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Args {
    values: [u64; MAX_ARGS],
}

impl Args {
    pub const fn empty() -> Self { Self { values: [0; MAX_ARGS] } }
    pub const fn from_array(values: [u64; MAX_ARGS]) -> Self { Self { values } }
    pub const fn get(&self, index: usize) -> Option<u64> {
        if index < MAX_ARGS { Some(self.values[index]) } else { None }
    }
    pub const fn ptr(&self, index: usize) -> Option<*const u8> {
        match self.get(index) { Some(value) if value != 0 => Some(value as *const u8), _ => None }
    }
    pub const fn mut_ptr(&self, index: usize) -> Option<*mut u8> {
        match self.get(index) { Some(value) if value != 0 => Some(value as *mut u8), _ => None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Return { pub value: i64 }

impl Return {
    pub const fn success(value: u64) -> Self { Self { value: value as i64 } }
    pub const fn error(error: Error) -> Self { Self { value: error as i64 } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub abi_version: u16,
    pub syscall: u64,
    pub args: Args,
}

impl Request {
    pub const fn new(syscall: u64, args: Args) -> Self {
        Self { abi_version: ABI_VERSION, syscall, args }
    }
    pub const fn decode_syscall(&self) -> Result<Syscall, Error> {
        if self.abi_version != ABI_VERSION { return Err(Error::NotSupported); }
        match Syscall::from_raw(self.syscall) {
            Some(syscall) => Ok(syscall),
            None => Err(Error::InvalidSyscall),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_syscall_decodes() {
        let request = Request::new(Syscall::ProcessSelf as u64, Args::empty());
        assert_eq!(request.decode_syscall(), Ok(Syscall::ProcessSelf));
    }

    #[test]
    fn unknown_syscall_is_rejected() {
        let request = Request::new(0xffff, Args::empty());
        assert_eq!(request.decode_syscall(), Err(Error::InvalidSyscall));
    }

    #[test]
    fn incompatible_abi_is_rejected() {
        let request = Request { abi_version: 99, syscall: Syscall::ProcessSelf as u64, args: Args::empty() };
        assert_eq!(request.decode_syscall(), Err(Error::NotSupported));
    }

    #[test]
    fn null_pointer_is_not_accepted_as_user_pointer() {
        let args = Args::from_array([0, 1, 2, 3, 4, 5]);
        assert!(args.ptr(0).is_none());
        assert!(args.ptr(1).is_some());
    }

    #[test]
    fn out_of_range_argument_is_rejected() {
        assert!(Args::empty().get(MAX_ARGS).is_none());
    }

    #[test]
    fn errors_are_stable() {
        assert_eq!(Error::InvalidArgument as i64, -2);
        assert_eq!(Return::success(7).value, 7);
        assert_eq!(Return::error(Error::PermissionDenied).value, -4);
    }
}
