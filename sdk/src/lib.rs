#![no_std]
#![allow(dead_code)]
//! ShivaCore SDK boundary: capabilities, IPC and attestation handles.
//! No AI/LLM or Web3 logic belongs in this kernel-facing crate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityId(pub u64);
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChannelId(pub u64);
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AttestationId(pub u64);
pub trait CapabilityResolver { fn resolve(&self, id: CapabilityId) -> bool; }
pub trait IpcEndpoint { fn channel(&self) -> ChannelId; }
