//! Capability-gated identity key service boundary for GlobusOS.
//!
//! The service accepts an opaque capability context. The authoritative capability
//! decision is made by ShivaCore's kernel capability table before a hardware-backed
//! identity-key operation is permitted.

#![allow(dead_code)]

extern crate alloc;

use shivacore::capability::{CapabilityTable, CapId, Pid, ResourceType, Rights};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOperation {
    Encrypt,
    Decrypt,
    Hmac,
    Sign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityContext {
    pub caller: CallerId,
    pub capability: CapabilityRef,
    pub operation: KeyOperation,
}

pub trait CapabilityAuthorizer {
    fn authorize(&self, context: CapabilityContext, handle: KeyHandle) -> bool;
}

/// Adapter to the kernel's authoritative capability table.
pub struct KernelCapabilityAuthorizer<'a> {
    table: &'a CapabilityTable,
}

impl<'a> KernelCapabilityAuthorizer<'a> {
    pub const fn new(table: &'a CapabilityTable) -> Self {
        Self { table }
    }
}

impl CapabilityAuthorizer for KernelCapabilityAuthorizer<'_> {
    fn authorize(&self, context: CapabilityContext, handle: KeyHandle) -> bool {
        if context.capability.0 != handle.0 || context.caller.0 > u32::MAX as u64 {
            return false;
        }
        self.table.check_any(Pid(context.caller.0 as u32), context.capability.0, Rights::READ)
            && self.table
                .get(CapId(context.capability.0))
                .map(|cap| cap.resource_type == ResourceType::IdentityKey && cap.resource_id == handle.0)
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyError {
    InvalidHandle,
    NotAuthorized,
    KeyUnavailable,
    HardwareFailure,
}

/// Non-exporting backend contract. The key itself is never returned.
pub trait IdentityKeyBackend {
    fn cryptographic_operation(
        &self,
        handle: KeyHandle,
        operation: KeyOperation,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, IdentityKeyError>;
}

pub struct IdentityKeyService<B, A> {
    backend: B,
    authorizer: A,
}

impl<B, A> IdentityKeyService<B, A> {
    pub const fn new(backend: B, authorizer: A) -> Self {
        Self { backend, authorizer }
    }
}

impl<B: IdentityKeyBackend, A: CapabilityAuthorizer> IdentityKeyService<B, A> {
    pub fn cryptographic_operation(
        &self,
        handle: KeyHandle,
        context: CapabilityContext,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, IdentityKeyError> {
        if !self.authorizer.authorize(context, handle) {
            return Err(IdentityKeyError::NotAuthorized);
        }
        self.backend
            .cryptographic_operation(handle, context.operation, input, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;
    impl IdentityKeyBackend for TestBackend {
        fn cryptographic_operation(
            &self,
            handle: KeyHandle,
            operation: KeyOperation,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, IdentityKeyError> {
            if handle.0 == 0 { return Err(IdentityKeyError::InvalidHandle); }
            if !matches!(operation, KeyOperation::Hmac) { return Err(IdentityKeyError::HardwareFailure); }
            if output.len() < input.len() { return Err(IdentityKeyError::HardwareFailure); }
            output[..input.len()].copy_from_slice(input);
            Ok(input.len())
        }
    }

    fn context(caller: u64, capability: u64) -> CapabilityContext {
        CapabilityContext { caller: CallerId(caller), capability: CapabilityRef(capability), operation: KeyOperation::Hmac }
    }

    fn provisioned_service() -> (IdentityKeyService<TestBackend, KernelCapabilityAuthorizer<'static>>, CapabilityRef) {
        let table = Box::leak(Box::new(CapabilityTable::new()));
        let cap = table.create(Pid(7), ResourceType::IdentityKey, 42, Rights::READ);
        (IdentityKeyService::new(TestBackend, KernelCapabilityAuthorizer::new(table)), CapabilityRef(cap.0))
    }

    #[test]
    fn kernel_capability_authorizes_matching_identity_key() {
        let (service, capability) = provisioned_service();
        let mut out = [0u8; 4];
        assert_eq!(service.cryptographic_operation(KeyHandle(42), context(7, capability.0), b"test", &mut out), Ok(4));
        assert_eq!(&out, b"test");
    }

    #[test]
    fn wrong_caller_is_rejected() {
        let (service, capability) = provisioned_service();
        let mut out = [0u8; 4];
        assert_eq!(service.cryptographic_operation(KeyHandle(42), context(8, capability.0), b"test", &mut out), Err(IdentityKeyError::NotAuthorized));
    }

    #[test]
    fn wrong_resource_is_rejected() {
        let (service, capability) = provisioned_service();
        let mut out = [0u8; 4];
        assert_eq!(service.cryptographic_operation(KeyHandle(41), context(7, capability.0), b"test", &mut out), Err(IdentityKeyError::NotAuthorized));
    }

    #[test]
    fn forged_capability_id_is_rejected() {
        let (service, _) = provisioned_service();
        let mut out = [0u8; 4];
        assert_eq!(service.cryptographic_operation(KeyHandle(42), context(7, 999_999), b"test", &mut out), Err(IdentityKeyError::NotAuthorized));
    }
}
