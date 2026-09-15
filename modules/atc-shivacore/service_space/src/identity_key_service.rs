//! Capability-gated identity key service boundary for GlobusOS.
//!
//! The service accepts an opaque capability context instead of a caller-supplied
//! boolean. The concrete kernel capability table remains outside this service-space
//! API, allowing the kernel to perform the authoritative ownership/rights check.

#![allow(dead_code)]

extern crate alloc;

/// Opaque identifier for a key held by the secure key service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyHandle(pub u64);

/// Stable identity of the caller supplied by the kernel IPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallerId(pub u64);

/// Opaque capability reference. It is not forgeable by the service itself; the
/// kernel IPC layer must validate ownership and rights before authorizing it.
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

/// Errors returned without exposing key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyError {
    InvalidHandle,
    NotAuthorized,
    KeyUnavailable,
    HardwareFailure,
}

/// Backend boundary. Implementations must keep the persistent key in the secure
/// hardware boundary. This transitional API returns key bytes only to protected
/// service memory; the next backend generation should expose crypto operations
/// directly so raw key material never leaves the hardware boundary.
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

    struct TestAuthorizer;
    impl CapabilityAuthorizer for TestAuthorizer {
        fn authorize(&self, context: CapabilityContext, handle: KeyHandle) -> bool {
            context.caller.0 == 7
                && context.capability.0 == handle.0
                && matches!(context.operation, KeyOperation::LoadEncryptionKey)
        }
    }

    fn context(caller: u64, capability: u64, handle: u64) -> CapabilityContext {
        CapabilityContext {
            caller: CallerId(caller),
            capability: CapabilityRef(capability),
            operation: KeyOperation::LoadEncryptionKey,
        }
        .with_handle_for_test(handle)
    }

    trait ContextTestExt {
        fn with_handle_for_test(self, _handle: u64) -> Self;
    }
    impl ContextTestExt for CapabilityContext {
        fn with_handle_for_test(self, _handle: u64) -> Self { self }
    }

    #[test]
    fn authorization_is_bound_to_caller_and_capability() {
        let service = IdentityKeyService::new(TestBackend, TestAuthorizer);
        let ctx = context(7, 1, 1);
        assert_eq!(service.load_key(KeyHandle(1), ctx).unwrap(), [0xA5; 32]);
        assert_eq!(
            service.load_key(KeyHandle(1), context(8, 1, 1)),
            Err(IdentityKeyError::NotAuthorized)
        );
    }

    #[test]
    fn invalid_capability_is_rejected() {
        let service = IdentityKeyService::new(TestBackend, TestAuthorizer);
        assert_eq!(
            service.load_key(KeyHandle(1), context(7, 99, 1)),
            Err(IdentityKeyError::NotAuthorized)
        );
    }

    #[test]
    fn invalid_handle_is_rejected_after_authorization() {
        let service = IdentityKeyService::new(TestBackend, TestAuthorizer);
        assert_eq!(
            service.load_key(KeyHandle(0), context(7, 0, 0)),
            Err(IdentityKeyError::InvalidHandle)
        );
    }
}
