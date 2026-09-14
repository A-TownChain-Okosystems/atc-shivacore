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

**Project:** `atc-shivacore`  
**Organization:** `A-TownChain-Okosystems`  
**Status:** `development`  
**Version:** `0.1.0`  
**Production:** `NOT_READY`  
**License:** `Apache-2.0`

## Role and Scope

ShivaCore stellt den wiederverwendbaren Kernel-/TCB-Baustein des Ökosystems bereit. Der normative Kernelvertrag ist OS-neutral und konzentriert sich auf Isolation, Capability-basierte Autorisierung, Scheduling, Memory, IPC, Timer und die erforderlichen Low-Level-Primitiven.

**In scope:**
- Rust/no_std-Microkernel und Capability-Schutzmodell
- CSpace/Capability Management
- Scheduling und Kernel-Lifecycle
- Memory Management und IPC
- Architektur-HAL für die tatsächlich implementierten Targets
- Boot-/Target-Unterstützung gemäß dokumentiertem Boot Contract

**Out of scope:**
- GlobusOS-Userspace und Systemdienste (`globus-os`)
- Aurora AI (`aurora-ai`)
- A-TownChain-Protokoll und Chain-State
- ATCLang-Contracts und ATC-VM
- Game-/GameFi-Anwendungen (`genesis-engine`, `genesis-chronicles`)

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

Ein anderes Betriebssystem kann denselben Kernel verwenden:

```text
OS-A userspace ──┐
OS-B userspace ──┼──→ ShivaCore ──→ Hardware
GlobusOS ────────┘
```

Die Wiederverwendbarkeit wird durch den Kernel-, HAL-, ABI- und Boot-Vertrag definiert, nicht durch eine README-Behauptung.

## Current Implementation Boundary

Das Repository enthält neben dem minimalen TCB weitere historische/experimentelle Kernel-Module. Diese sind nicht automatisch Teil des normativen Microkernel-Vertrags. OS-, Blockchain- oder AI-Semantik darf nicht ohne Governance und TCB-Review in den Kernvertrag aufgenommen werden.

## Requirements

- Rust 1.98.1 oder neuer, sofern der aktuelle Workspace dies voraussetzt
- Cargo und Build-Essentials
- unterstützte `x86_64-unknown-none` bzw. `aarch64-unknown-none` Targets, sofern vom jeweiligen Modul aktiviert
- Python 3.10+ für vorhandene Workspace-Hilfsskripte

## Installation

```bash
git clone https://github.com/A-TownChain-Okosystems/atc-shivacore.git
cd atc-shivacore
cargo build --workspace
```

## Boot / Image Builder

Der Repository-Workspace enthält einen separaten Boot-Image-Builder für BIOS/UEFI. Die konkrete Target-Unterstützung muss durch aktuelle CI-Evidence bestätigt werden.

```bash
cargo run --bin boot --manifest-path modules/atc-shivacore/boot/Cargo.toml -- <kernel-elf> <output-dir>
```

## Testing

```bash
cargo test --workspace
```

Testergebnisse gelten immer für den jeweiligen Commit und werden nicht als dauerhafte Testzahl im README garantiert.

## Documentation

- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`STATUS.md`](STATUS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`SECURITY.md`](SECURITY.md)
- [`docs/specs/SHIVA-KERNEL-REUSE-001.md`](docs/specs/SHIVA-KERNEL-REUSE-001.md) — Reusable Kernel Contract
- [`docs/specs/SHIVA-HAL-001.md`](docs/specs/SHIVA-HAL-001.md) — Hardware Abstraction Layer
- [`docs/specs/SHIVA-ABI-001.md`](docs/specs/SHIVA-ABI-001.md) — Kernel/Userspace ABI
- [`docs/specs/SHIVA-BOOT-001.md`](docs/specs/SHIVA-BOOT-001.md) — Boot Contract

## Security

ShivaCore ist sicherheitskritische Infrastruktur. Sicherheitslücken nicht öffentlich über GitHub Issues veröffentlichen; den in `SECURITY.md` definierten Disclosure-Prozess verwenden.

## Development and Governance

- Änderungen folgen `ATC-STD-000` und dem aktuellen ATC-Governance-Prozess.
- Architekturänderungen mit TCB-Auswirkung benötigen dokumentierte Governance-/Review-Evidence.
- Conventional Commits verwenden.
- Integration in `a-townchain-os` ist eine Integrationsaufgabe; ShivaCore bleibt als wiederverwendbarer Kernel eigenständig.
- Neue family-scoped Standard-IDs verwenden `ATC-STD-F{family}-{sequence}`; historische IDs werden nicht stillschweigend umnummeriert.

## Blockchain Boundary

```text
ATCLang → ATC-VM → A-TownChain
```

ShivaCore stellt keine Chain-Semantik und keine feste Chain-ID bereit.

## License

Apache-2.0. Siehe [`LICENSE`](LICENSE).

## AI Agent Instructions

Vor Änderungen mindestens `AGENTS.md`, `AGENT_MANIFEST.md`, `ARCHITECTURE.md`, `STATUS.md` und `ROADMAP.md` prüfen. Änderungen am TCB, Capability-Modell, Bootpfad oder Sicherheitsgrenzen benötigen besonders sorgfältige Tests und Governance-Evidence.
