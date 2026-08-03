// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// K-Sprint 20 — Contract-Call-Integration
use alloc::vec::Vec; use alloc::string::{String,ToString}; use alloc::sync::Arc;
use crate::mempool::{Transaction,TxType};
use crate::vm::{VmEngine,ExecResult,VmError};
use crate::security::simple_hash;
pub struct ContractExecutor{vm:Arc<VmEngine>,state:Arc<crate::mempool::StateDb>}
impl ContractExecutor{pub fn new(vm:Arc<VmEngine>,state:Arc<crate::mempool::StateDb>)->Self{ContractExecutor{vm,state}}
pub fn process_deploy(&self,tx:&Transaction)->Result<String,ContractError>{if tx.tx_type!=TxType::ContractDeploy{return Err(ContractError::WrongTxType);}if tx.payload.len()<4{return Err(ContractError::InvalidPayload("too short".into()));}let bl=u32::from_be_bytes([tx.payload[0],tx.payload[1],tx.payload[2],tx.payload[3]]) as usize;if tx.payload.len()<4+bl{return Err(ContractError::InvalidPayload("length mismatch".into()));}let bytecode=tx.payload[4..4+bl].to_vec();let mut ai=Vec::new();ai.extend_from_slice(tx.sender_did.as_bytes());ai.extend_from_slice(&tx.nonce.to_be_bytes());ai.extend_from_slice(&tx.poh_hash);let ah=simple_hash(&ai);let addr=format!("did:contract:{}",hex(&ah[..8]));self.vm.deploy(addr.clone(),bytecode,tx.sender_did.clone());Ok(addr)}
pub fn process_call(&self,tx:&Transaction)->Result<ExecResult,ContractError>{if tx.tx_type!=TxType::ContractCall{return Err(ContractError::WrongTxType);}if tx.payload.len()<2{return Err(ContractError::InvalidPayload("too short".into()));}let al=u16::from_be_bytes([tx.payload[0],tx.payload[1]]) as usize;if tx.payload.len()<2+al{return Err(ContractError::InvalidPayload("addr mismatch".into()));}let addr=String::from_utf8_lossy(&tx.payload[2..2+al]).to_string();if!self.vm.registry.exists(&addr){return Err(ContractError::ContractNotFound(addr));}let r=self.vm.call(&addr,tx.sender_did.clone(),Some(tx.gas_limit));if!r.success{return Err(ContractError::ExecutionFailed(r));}Ok(r)}
pub fn process_tx(&self,tx:&Transaction)->Result<TxProcessingResult,ContractError>{match tx.tx_type{TxType::ContractDeploy=>{let a=self.process_deploy(tx)?;Ok(TxProcessingResult::Deployed(a))}TxType::ContractCall=>{let r=self.process_call(tx)?;Ok(TxProcessingResult::Called(r))}_=>Err(ContractError::NotAContractTx)}}pub fn vm(&self)->&Arc<VmEngine>{&self.vm}}
#[derive(Debug,Clone)] pub enum TxProcessingResult{Deployed(String),Called(ExecResult)}
#[derive(Debug,Clone,PartialEq,Eq)] pub enum ContractError{WrongTxType,NotAContractTx,InvalidPayload(String),ContractNotFound(String),ExecutionFailed(ExecResult)}
pub fn build_deploy_payload(bytecode:&[u8])->Vec<u8>{let mut p=Vec::new();p.extend_from_slice(&(bytecode.len() as u32).to_be_bytes());p.extend_from_slice(bytecode);p}
pub fn build_call_payload(addr:&str,call_data:&[u8])->Vec<u8>{let mut p=Vec::new();let ab=addr.as_bytes();p.extend_from_slice(&(ab.len() as u16).to_be_bytes());p.extend_from_slice(ab);p.extend_from_slice(call_data);p}
fn hex(bytes:&[u8])->String{let mut s=String::new();for&b in bytes{s.push_str(&format!("{:02x}",b));}s}
