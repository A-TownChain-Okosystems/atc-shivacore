<!--
atc:
  standard: ATC-STD-README-001
  version: 1.0.0
repository:
  id: ATC-REPO-CORE-001
  name: atc-shivacore
  type: software
  status: development
ownership:
  organization: A-TownChain-Okosystems
technology:
  primary_language: Rust
governance:
  security_class: S4
  criticality: C1
-->

# ATC ShivaCore

> Capability-basierter Rust/no_std-Microkernel als wiederverwendbare sicherheitskritische Kernel-Basis für GlobusOS. ShivaCore ist kein Blockchain-, AI- oder Game-Layer.

**ATC COMPLIANCE: R4 · DEVELOPMENT · Score pending CI · Standard 1.0.1 · GATE: PENDING**

**Project:** `atc-shivacore`  
**Organization:** `A-TownChain-Okosystems`  
**Status:** `development`  
**Version:** `0.1.0`  
**Production:** `NOT_READY`  
**License:** `Apache-2.0`

## Purpose

ShivaCore provides the reusable kernel/TCB foundation for GlobusOS and other operating-system integrations. The normative kernel contract is OS-neutral and focuses on isolation, capability-based authorization, scheduling, memory, IPC, timers, and low-level architecture primitives.

## Scope

**In scope:**
- Rust/no_std microkernel and capability protection model
- CSpace/capability management
- Scheduling and kernel lifecycle
- Memory management and IPC
- Architecture HAL for implemented targets
- Boot/target support according to the documented boot contract

**Out of scope:** GlobusOS userspace and services, Aurora AI, A-TownChain protocol/state, ATCLang/ATC-VM, and Game/GameFi applications.

## Architecture

```text
Firmware / Bootloader
        ↓
Architecture HAL
        ↓
ShivaCore Microkernel / TCB
  ├─ Capabilities / CSpace
  ├─ Memory / Address Spaces
  ├─ Scheduling / Threads
  ├─ IPC / Endpoints
  └─ Timers / Traps
        ↓
Kernel-facing OS services
        ↓
GlobusOS userspace / Aurora / applications
```

ShivaCore can be reused by another OS when the kernel, HAL, ABI, and boot contracts are satisfied.

## Features

- Capability-based resource authorization and delegation
- Process address-space isolation policy
- x86_64 paging flag translation with user/kernel separation
- Physical-frame allocation integration through the bootloader memory map
- Scheduling, IPC, timers, and kernel lifecycle primitives
- BIOS/UEFI boot-image support through the dedicated boot crate

## Repository Structure

```text
modules/atc-shivacore/kernel/   Kernel and TCB implementation
modules/atc-shivacore/boot/     BIOS/UEFI image builder
docs/                            Specifications and contracts
.github/                         CI/CD and repository automation
.atc/                            Machine-readable repository governance
```

## Installation

Requirements: Rust 1.98.1+ where required by the current workspace, Cargo, build essentials, and Python 3.10+ for repository tooling.

```bash
git clone https://github.com/A-TownChain-Okosystems/atc-shivacore.git
cd atc-shivacore
cargo build --workspace
```

## Development

Follow `AGENTS.md` and `AGENT_MANIFEST.md` before changing the kernel. TCB, capability, boot, paging, ABI, or isolation changes require focused review and evidence.

Use Conventional Commits. Do not treat experimental or historical kernel modules as part of the normative TCB without governance approval.

## Testing

```bash
cargo test --workspace
```

Test results are commit-specific and are not represented as permanent test-count claims in this README.

## Security

ShivaCore is security-critical infrastructure. Do not disclose vulnerabilities through public GitHub issues. Follow `SECURITY.md` for coordinated disclosure.

The kernel must maintain capability isolation, user/kernel memory separation, explicit authority, and fail-closed security boundaries.

## Roadmap

1. Complete architecture-neutral memory-isolation contract.
2. Complete x86_64 page-table and physical-frame integration.
3. Add process-specific address-space lifecycle and controlled unmapping.
4. Enforce page-fault isolation and capability-bound memory operations.
5. Validate reusable kernel/ABI/boot contracts on supported architectures.

Current repository status remains `development` / `NOT_READY`; roadmap completion must not be inferred from documentation alone.

## Versioning

The repository version is `0.1.0` and remains development-stage. Standard changes follow the ATC governance lifecycle. New family-scoped standard IDs use `ATC-STD-F{family}-{sequence}`; historical IDs are never silently renumbered.

## Documentation

- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`STATUS.md`](STATUS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`SECURITY.md`](SECURITY.md)
- [`docs/specs/SHIVA-KERNEL-REUSE-001.md`](docs/specs/SHIVA-KERNEL-REUSE-001.md)
- [`docs/specs/SHIVA-HAL-001.md`](docs/specs/SHIVA-HAL-001.md)
- [`docs/specs/SHIVA-ABI-001.md`](docs/specs/SHIVA-ABI-001.md)
- [`docs/specs/SHIVA-BOOT-001.md`](docs/specs/SHIVA-BOOT-001.md)

## Blockchain Boundary

```text
ATCLang → ATC-VM → A-TownChain
```

ShivaCore provides no chain semantics and no fixed chain ID.

## License

Apache-2.0. See [`LICENSE`](LICENSE).

## AI Agent Instructions

Before changes, inspect `AGENTS.md`, `AGENT_MANIFEST.md`, `ARCHITECTURE.md`, `STATUS.md`, and `ROADMAP.md`. TCB, capability, boot-path, paging, ABI, and security-boundary changes require careful tests and governance evidence.
