//! Capability-gated identity key service boundary for GlobusOS.
//!
//! The service accepts an opaque capability context. The authoritative capability
//! decision is made by ShivaCore's kernel capability table before a hardware-backed
//! identity-key operation is permitted.

#![allow(dead_code)]

extern crate alloc;

use shivacore::capability::{CapabilityTable, ResourceType, Rights};

/// Opaque identifier for a key held by the secure key service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyHandle(pub u64);

/// Stable identity of the caller supplied by the kernel IPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallerId(pub u64);

/// Opaque capability reference supplied by the kernel IPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityRef(pub u64);

/// Operations that may be requested from the identity key service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOperation {
    LoadEncryptionKey,
}

/// Authorization context carried across the trusted service boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityContext {
    pub caller: CallerId,
    pub capability: CapabilityRef,
    pub operation: KeyOperation,
}

/// Authoritative capability decision supplied by the kernel.
pub trait CapabilityAuthorizer {
    fn authorize(&self, context: CapabilityContext, handle: KeyHandle) -> bool;
}

/// Adapter from the service-space authorization boundary to ShivaCore's real
/// capability table. Identity-key handles are represented as kernel resources,
/// never as caller-forged authorization booleans.
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
        if !matches!(context.operation, KeyOperation::LoadEncryptionKey) {
            return false;
        }
        if context.capability.0 != handle.0 {
            return false;
        }
        if context.caller.0 > u32::MAX as u64 {
            return false;
        }
        self.table.check_any(
            shivacore::capability::Pid(context.caller.0 as u32),
            context.capability.0,
            Rights::READ,
        ) && self.table
            .get(shivacore::capability::CapId(context.capability.0))
            .is_some_and(|cap| cap.resource_type == ResourceType::IdentityKey && cap.resource_id == handle.0)
    }
}

/// Errors returned without exposing key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyError {
    InvalidHandle,
    NotAuthorized,
    KeyUnavailable,
    HardwareFailure,
}

/// Transitional backend boundary. Production hardware implementations must bind the
/// handle to a TPM, TEE, or secure-element object. The raw-key return remains only for
/// the current migration path; the next API removes key export entirely.
pub trait IdentityKeyBackend {
    fn load_key(&self, handle: KeyHandle) -> Result<[u8; 32], IdentityKeyError>;
}

/// Capability-gated service facade.
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
    pub fn load_key(
        &self,
        handle: KeyHandle,
        context: CapabilityContext,
    ) -> Result<[u8; 32], IdentityKeyError> {
        if !self.authorizer.authorize(context, handle) {
            return Err(IdentityKeyError::NotAuthorized);
        }
        self.backend.load_key(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;
    impl IdentityKeyBackend for TestBackend {
        fn load_key(&self, handle: KeyHandle) -> Result<[u8; 32], IdentityKeyError> {
            if handle.0 == 0 {
                return Err(IdentityKeyError::InvalidHandle);
            }
            Ok([0xA5; 32])
        }
    }

    fn context(caller: u64, capability: u64) -> CapabilityContext {
        CapabilityContext {
            caller: CallerId(caller),
            capability: CapabilityRef(capability),
            operation: KeyOperation::LoadEncryptionKey,
        }
    }

    fn provisioned_service() -> (IdentityKeyService<TestBackend, KernelCapabilityAuthorizer<'static>>, CapabilityRef) {
        let table = Box::leak(Box::new(CapabilityTable::new()));
        let cap = table.create(
            shivacore::capability::Pid(7),
            ResourceType::IdentityKey,
            42,
            Rights::READ,
        );
        (IdentityKeyService::new(TestBackend, KernelCapabilityAuthorizer::new(table)), CapabilityRef(cap.0))
    }

    #[test]
    fn kernel_capability_authorizes_matching_identity_key() {
        let (service, capability) = provisioned_service();
        assert_eq!(
            service.load_key(KeyHandle(42), context(7, capability.0)),
            Ok([0xA5; 32])
        );
    }

    #[test]
    fn wrong_caller_is_rejected() {
        let (service, capability) = provisioned_service();
        assert_eq!(
            service.load_key(KeyHandle(42), context(8, capability.0)),
            Err(IdentityKeyError::NotAuthorized)
        );
    }

    #[test]
    fn wrong_resource_is_rejected() {
        let (service, capability) = provisioned_service();
        assert_eq!(
            service.load_key(KeyHandle(41), context(7, capability.0)),
            Err(IdentityKeyError::NotAuthorized)
        );
    }

    #[test]
    fn forged_capability_id_is_rejected() {
        let (service, _) = provisioned_service();
        assert_eq!(
            service.load_key(KeyHandle(42), context(7, 999_999)),
            Err(IdentityKeyError::NotAuthorized)
        );
    }
}
