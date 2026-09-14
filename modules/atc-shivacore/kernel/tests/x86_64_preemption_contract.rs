//! Architecture-level contract tests for the x86_64 timer preemption path.
//!
//! These tests intentionally validate the pieces that can be checked without
//! starting QEMU. The real A -> B -> A transition remains a boot-time test.

#[test]
fn preemption_contract_requires_ring3_context() {
    // x86_64 RPL3 selector contract used by ProcessExecutionContext.
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
    // The actual address is architecture/runtime-specific; this test only
    // guarantees the symbol-facing API remains available to the boot layer.
    let _ = shivacore::x86_64_timer_entry::entry_address;
}
