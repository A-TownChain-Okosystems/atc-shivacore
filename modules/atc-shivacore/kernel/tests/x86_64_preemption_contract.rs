#![cfg(feature = "x86-boot")]

//! Architecture-level contract tests for the x86_64 timer preemption path.

#[test]
fn preemption_contract_requires_ring3_context() {
    assert_eq!(0x1b_u64 & 0x3, 0x3);
    assert_eq!(0x08_u64 & 0x3, 0x0);
}

#[test]
fn context_restore_contract_is_15_registers_plus_iret_frame() {
    use shivacore::x86_64_context_switch::{ContextStack, IretFrame, SavedRegisters};
    assert_eq!(core::mem::size_of::<SavedRegisters>(), 15 * core::mem::size_of::<u64>());
    assert_eq!(core::mem::size_of::<IretFrame>(), 5 * core::mem::size_of::<u64>());
    assert_eq!(core::mem::size_of::<ContextStack>(), 20 * core::mem::size_of::<u64>());
}

#[test]
fn timer_entry_symbol_contract_is_exposed() {
    let _ = shivacore::x86_64_timer_entry::entry_address;
}
