//! ShivaCore Kernel — Dezentrale Knoten-Identitaet (DID) (Rust).
//!
//! Portiert did.py (Python, 08.07.2026) nach Rust.
//! Dezentrale Identitaet mit kryptographischen Signaturen.
//! Die eigentliche Kryptografie ist ueber ein Trait-Interface abstrahiert
//! — echte Ed25519 (oder Hardware-Enklave) wird spaeter eingehaengt,
//! ohne die Algorithmus-Logik zu aendern.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::clone::Clone;

/// Dezentrale Identitaet: did:shivacore:<base64-public-key>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Did {
    pub value: String,
}

impl Did {
    pub fn new(value: &str) -> Self { Self { value: value.to_string() } }
    pub fn as_str(&self) -> &str { &self.value }
}

/// Trait fuer kryptographische Operationen.
/// Implementierungen: Ed25519Signer (Software), spaeter HardwareEnclaveSigner.
pub trait CryptoProvider {
    fn did(&self) -> &Did;
    fn sign(&self, payload: &[u8]) -> Vec<u8>;
    fn verify(did: &Did, payload: &[u8], signature: &[u8]) -> bool;
}

/// Software-basierter Signer fuer Tests (deterministische Pseudo-Signatur).
/// In Produktion: Ed25519Signer mit ed25519-dalek oder Hardware-Enklave.
#[derive(Debug, Clone)]
pub struct SoftwareSigner {
    did: Did,
    private_key: Vec<u8>,  // Simuliert — in Produktion echtes Ed25519
}

impl SoftwareSigner {
    /// Erzeugt eine neue Knoten-Identitaet mit deterministischem Schluesselpaar.
    pub fn new(name: &str) -> Self {
        // Deterministischer "Schluessel" aus Name (fuer reproduzierbare Tests)
        let private = name.as_bytes().to_vec();
        let public_b64 = simulate_encode(&private);
        let did = Did::new(&format!("did:shivacore:{}", public_b64));
        Self { did, private_key: private }
    }

    /// Erzeugt einen Signer mit bekanntem privaten Schluessel (fuer Tests)
    pub fn with_key(private_key: &[u8]) -> Self {
        let public_b64 = simulate_encode(private_key);
        let did = Did::new(&format!("did:shivacore:{}", public_b64));
        Self { did, private_key: private_key.to_vec() }
    }
}

impl CryptoProvider for SoftwareSigner {
    fn did(&self) -> &Did { &self.did }
    fn sign(&self, payload: &[u8]) -> Vec<u8> {
        // Deterministische Pseudo-Signatur: HMAC-artig (key XOR payload hash)
        // In Produktion: Ed25519 sign(payload, private_key)
        let mut sig = Vec::with_capacity(payload.len() + self.private_key.len());
        for (i, b) in payload.iter().enumerate() {
            sig.push(b ^ self.private_key[i % self.private_key.len()]);
        }
        // Append key hash for verification identity
        sig.extend_from_slice(&simulate_encode(&self.private_key).into_bytes());
        sig
    }

    fn verify(did: &Did, payload: &[u8], signature: &[u8]) -> bool {
        // Extrahiere den oeffentlichen Schluessel aus der DID
        let prefix = "did:shivacore:";
        if !did.value.starts_with(prefix) { return false; }
        let public_b64 = &did.value[prefix.len()..];
        let key_bytes = match simulate_decode(public_b64) {
            Some(k) => k,
            None => return false,
        };

        // Signatur muss laenger sein als der Payload (Payload-XOR + Key-Hash)
        if signature.len() < payload.len() { return false; }
        let key_hash_start = payload.len();
        let key_hash = &signature[key_hash_start..];

        // Verifiziere: Key-Hash am Ende muss zum oeffentlichen Schluessel passen
        let expected_hash = simulate_encode(&key_bytes).into_bytes();
        if key_hash != expected_hash.as_slice() { return false; }

        // Verifiziere: XOR-Decodierung ergibt konsistenten Payload
        for (i, b) in payload.iter().enumerate() {
            let decoded = signature[i] ^ key_bytes[i % key_bytes.len()];
            if decoded != *b { return false; }
        }
        true
    }
}

/// Hilfsfunktion: deterministische "Base64" (fuer Tests)
fn simulate_encode(data: &[u8]) -> String {
    let mut result = String::new();
    for b in data {
        result.push_str(&format!("{:02x}", b));
    }
    result
}

fn simulate_decode(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 { return None; }
    let mut result = Vec::new();
    let bytes = hex.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        result.push((hi << 4) | lo);
        i += 2;
    }
    Some(result)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_format() {
        let signer = SoftwareSigner::new("alice");
        assert!(signer.did().value.starts_with("did:shivacore:"));
    }

    #[test]
    fn test_sign_and_verify() {
        let alice = SoftwareSigner::new("alice");
        let payload = b"hello shivacore";
        let sig = alice.sign(payload);
        assert!(SoftwareSigner::verify(alice.did(), payload, &sig));
    }

    #[test]
    fn test_verify_rejects_wrong_signer() {
        let alice = SoftwareSigner::new("alice");
        let bob = SoftwareSigner::new("bob");
        let payload = b"hello shivacore";
        let sig = alice.sign(payload);
        // Bob's DID ≠ Alice's DID → Verifikation schlaegt fehl
        assert!(!SoftwareSigner::verify(bob.did(), payload, &sig));
    }

    #[test]
    fn test_verify_rejects_tampered_payload() {
        let alice = SoftwareSigner::new("alice");
        let sig = alice.sign(b"original payload");
        assert!(!SoftwareSigner::verify(alice.did(), b"tampered payload", &sig));
    }

    #[test]
    fn test_verify_rejects_tampered_signature() {
        let alice = SoftwareSigner::new("alice");
        let payload = b"hello";
        let mut sig = alice.sign(payload);
        sig[0] ^= 0xFF; // Veraendere ein Byte
        assert!(!SoftwareSigner::verify(alice.did(), payload, &sig));
    }

    #[test]
    fn test_did_equality() {
        let a1 = Did::new("did:shivacore:abc123");
        let a2 = Did::new("did:shivacore:abc123");
        let b = Did::new("did:shivacore:def456");
        assert_eq!(a1, a2);
        assert_ne!(a1, b);
    }
}
