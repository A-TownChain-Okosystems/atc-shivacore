//! Hardware-backed identity-key HAL boundary.
//!
//! Production implementations keep private key material inside the TPM, TEE, or
//! secure element. The API is operation-based: callers submit data and receive
//! only the cryptographic result, never the private key bytes.

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HardwareKeyHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyOperation {
    Encrypt,
    Decrypt,
    Hmac,
    Sign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareKeyError {
    InvalidHandle,
    NotProvisioned,
    Unavailable,
    AccessDenied,
    UnsupportedOperation,
    InvalidInput,
    OutputTooSmall,
    HardwareFailure,
}

/// Preferred HAL contract. Key material never crosses this boundary.
pub trait IdentityKeyHal {
    fn cryptographic_operation(
        &self,
        handle: HardwareKeyHandle,
        operation: IdentityKeyOperation,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, HardwareKeyError>;
}

pub struct HardwareIdentityKeyBackend<H> {
    hal: H,
}

impl<H> HardwareIdentityKeyBackend<H> {
    pub const fn new(hal: H) -> Self { Self { hal } }
}

impl<H: IdentityKeyHal> super::identity_key_service::IdentityKeyBackend
    for HardwareIdentityKeyBackend<H>
{
    fn cryptographic_operation(
        &self,
        handle: super::identity_key_service::KeyHandle,
        operation: super::identity_key_service::KeyOperation,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, super::identity_key_service::IdentityKeyError> {
        let operation = match operation {
            super::identity_key_service::KeyOperation::Encrypt => IdentityKeyOperation::Encrypt,
            super::identity_key_service::KeyOperation::Decrypt => IdentityKeyOperation::Decrypt,
            super::identity_key_service::KeyOperation::Hmac => IdentityKeyOperation::Hmac,
            super::identity_key_service::KeyOperation::Sign => IdentityKeyOperation::Sign,
        };
        self.hal
            .cryptographic_operation(HardwareKeyHandle(handle.0), operation, input, output)
            .map_err(map_hardware_error)
    }
}

fn map_hardware_error(
    error: HardwareKeyError,
) -> super::identity_key_service::IdentityKeyError {
    use super::identity_key_service::IdentityKeyError;
    match error {
        HardwareKeyError::InvalidHandle => IdentityKeyError::InvalidHandle,
        HardwareKeyError::AccessDenied => IdentityKeyError::NotAuthorized,
        HardwareKeyError::NotProvisioned => IdentityKeyError::KeyUnavailable,
        HardwareKeyError::Unavailable
        | HardwareKeyError::UnsupportedOperation
        | HardwareKeyError::InvalidInput
        | HardwareKeyError::OutputTooSmall
        | HardwareKeyError::HardwareFailure => IdentityKeyError::HardwareFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHal;
    impl IdentityKeyHal for TestHal {
        fn cryptographic_operation(
            &self,
            handle: HardwareKeyHandle,
            operation: IdentityKeyOperation,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, HardwareKeyError> {
            if handle.0 == 0 { return Err(HardwareKeyError::InvalidHandle); }
            if !matches!(operation, IdentityKeyOperation::Hmac) { return Err(HardwareKeyError::UnsupportedOperation); }
            if output.len() < input.len() { return Err(HardwareKeyError::OutputTooSmall); }
            output[..input.len()].copy_from_slice(input);
            Ok(input.len())
        }
    }

    #[test]
    fn operation_returns_only_result() {
        let backend = HardwareIdentityKeyBackend::new(TestHal);
        let mut output = [0u8; 4];
        let written = super::super::identity_key_service::IdentityKeyBackend::cryptographic_operation(
            &backend,
            super::super::identity_key_service::KeyHandle(1),
            super::super::identity_key_service::KeyOperation::Hmac,
            b"test",
            &mut output,
        ).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&output, b"test");
    }

    #[test]
    fn invalid_handle_fails_closed() {
        let backend = HardwareIdentityKeyBackend::new(TestHal);
        let mut output = [0u8; 4];
        let result = super::super::identity_key_service::IdentityKeyBackend::cryptographic_operation(
            &backend,
            super::super::identity_key_service::KeyHandle(0),
            super::super::identity_key_service::KeyOperation::Hmac,
            b"test",
            &mut output,
        );
        assert_eq!(result, Err(super::super::identity_key_service::IdentityKeyError::InvalidHandle));
    }
}
