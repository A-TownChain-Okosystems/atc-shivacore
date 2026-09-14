//! QEMU E2E test specification for timer-driven ring-3 preemption.
//!
//! This test is intentionally a specification marker rather than a fake PASS:
//! it documents the serial protocol expected from the boot binary once the
//! runtime creates two real processes.

#[test]
fn qemu_serial_protocol_is_defined() {
    const START: &str = "E2E_PREEMPTION_START";
    const A: &str = "E2E_PREEMPTION_A";
    const B: &str = "E2E_PREEMPTION_B";
    const A_AGAIN: &str = "E2E_PREEMPTION_A_AGAIN";
    const PASS: &str = "E2E_PREEMPTION_PASS";

    let protocol = [START, A, B, A_AGAIN, PASS];
    assert_eq!(protocol[0], "E2E_PREEMPTION_START");
    assert_eq!(protocol[4], "E2E_PREEMPTION_PASS");
}
