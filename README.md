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

> Capability-basierter Rust/no_std-Microkernel als wiederverwendbare sicherheitskritische Kernel-Basis für GlobusOS. Das aktive Kernel-Source-of-Truth liegt inzwischen in `globus-os`; dieses Repository enthält unterstützendes ShivaCore-Material.

**Project:** `atc-shivacore`  
**Organization:** `A-TownChain-Okosystems`  
**Status:** `development`  
**Version:** `0.1.0`  
**Production:** `NOT_READY`  
**License:** `Apache-2.0`

## Role and Scope

ShivaCore stellt den wiederverwendbaren Kernel-/TCB-Vertrag des Ökosystems bereit. Der aktive kanonische Kernel-Quellbaum wird in `globus-os/modules/atc-shivacore/kernel/` gebaut und verifiziert. Dieses Repository ist kein zweiter kanonischer Kernel-Source-of-Truth.

**In scope:**
- Kernel-/TCB-Spezifikationen und Governance
- unterstützendes Kernel- und Boot-Material
- Wiederverwendungs-, HAL-, ABI- und Boot-Verträge
- Dokumentation und Audit-Nachweise

**Out of scope:**
- der kanonische aktive Kernel-Quellbaum (siehe `globus-os`)
- GlobusOS-Userspace und Systemdienste
- Aurora AI
- A-TownChain-Protokoll und Chain-State
- ATCLang-Contracts und ATC-VM
- Game-/GameFi-Anwendungen

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
GlobusOS platform integration
```

Die Wiederverwendbarkeit wird durch den Kernel-, HAL-, ABI- und Boot-Vertrag definiert. Änderungen am kanonischen Kernel müssen im `globus-os` Repository erfolgen, damit dieselbe CI- und Integrationskette den TCB prüft.

## Current Implementation Boundary

Historische oder experimentelle Kernel-Bestandteile in diesem Repository sind nicht automatisch Teil des normativen Microkernel-Vertrags. OS-, Blockchain- oder AI-Semantik darf nicht ohne Governance und TCB-Review in den Kernvertrag aufgenommen werden.

## Requirements

- Rust für vorhandene Rust-Tools und unterstützende Komponenten
- Python nur für explizit dokumentierte Hilfsskripte
- Git
- Für kanonische Kernel-Builds gelten die Toolchain- und Target-Anforderungen von `globus-os`

## Installation

Dieses Repository ist primär ein Governance-/Dokumentations- und Support-Repository. Der kanonische Kernel-Build erfolgt im `globus-os` Repository:

```bash
git clone https://github.com/A-TownChain-Okosystems/globus-os.git
cd globus-os
cargo test --workspace
```

Vor einem Build sind die aktuellen `globus-os` Manifest-, Target- und CI-Vorgaben maßgeblich.

## Testing

Kanonische Kernel-Tests müssen aus dem aktuellen `globus-os` CI-Lauf stammen. Historische Testzahlen in diesem Repository gelten nicht als permanente Verifikation.

## Documentation

- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`STATUS.md`](STATUS.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`SECURITY.md`](SECURITY.md)
- [`docs/specs/SHIVA-KERNEL-REUSE-001.md`](docs/specs/SHIVA-KERNEL-REUSE-001.md)
- [`docs/specs/SHIVA-HAL-001.md`](docs/specs/SHIVA-HAL-001.md)
- [`docs/specs/SHIVA-ABI-001.md`](docs/specs/SHIVA-ABI-001.md)
- [`docs/specs/SHIVA-BOOT-001.md`](docs/specs/SHIVA-BOOT-001.md)

## Security

ShivaCore ist sicherheitskritische Infrastruktur. Sicherheitslücken nicht öffentlich über GitHub Issues veröffentlichen; den in `SECURITY.md` definierten Disclosure-Prozess verwenden.

Die Verlagerung des kanonischen Kernel-Quellbaums in `globus-os` reduziert die Gefahr divergierender TCB-Implementierungen: Build, Tests, Lints und Integration werden an einer Stelle ausgeführt. Das ist eine Architekturkontrolle, aber kein Beweis vollständiger Angriffs- oder Malware-Immunität.

## Development and Governance

- Änderungen folgen `ATC-STD-000` und dem aktuellen ATC-Governance-Prozess.
- Architekturänderungen mit TCB-Auswirkung benötigen dokumentierte Governance-/Review-Evidence.
- Conventional Commits verwenden.
- Neue family-scoped Standard-IDs verwenden `ATC-STD-F{family}-{sequence}`.
- Der kanonische Kernel-Source-of-Truth darf nicht in diesem Repository dupliziert werden.

## Blockchain Boundary

```text
ATCLang → ATC-VM → A-TownChain
```

ShivaCore stellt keine Chain-Semantik und keine feste Chain-ID bereit.

## Roadmap

Die operative Kernel-Roadmap und Umsetzungsplanung werden am kanonischen Implementierungsort `globus-os` geführt. Dieses Repository dokumentiert nur ShivaCore-spezifische Verträge, Governance und Support-Material.

## Version

Die Repository-Version ist `0.1.0`. Änderungen am kanonischen Kernel werden über die Versions- und Release-Prozesse von `globus-os` nachgewiesen.

## Compliance

Der aktuelle Repository-Zustand wird als **development / NOT_READY** geführt. `APPROVED`, `IMPLEMENTED`, `AUDITED` und `PRODUCTION_READY` sind unabhängige Zustände und dürfen nicht aus Dokumentation allein abgeleitet werden.

## License

Apache-2.0. Siehe [`LICENSE`](LICENSE).

## AI Agent Instructions

Vor Änderungen mindestens `AGENTS.md`, `AGENT_MANIFEST.md`, `ARCHITECTURE.md`, `STATUS.md` und `ROADMAP.md` prüfen. TCB-, Capability-, Boot- oder Sicherheitsgrenzen dürfen nicht gegen die Canonical-Source-Regel verschoben werden.

## ShivaCore relocation

The canonical ShivaCore kernel source is `A-TownChain-Okosystems/globus-os/modules/atc-shivacore/kernel/`. This repository retains supporting ShivaCore tooling, specifications and governance material.


## Compliance
![ATC COMPLIANCE](https://img.shields.io/badge/ATC%20COMPLIANCE-R3%20%C2%B7%20ATC--STD--201%2F202%2F203-brightgreen)

## Architecture
## Installation
## Usage
## Configuration
## Development
## Testing
## Security
## Governance
## Contributing
## License
## Support


## Overview
ShivaCore is the operating-system userspace layer built on ShivaCore.

## Purpose
Provides the user-facing OS services, identity, wallet, applications and system integration required by the A-TownChain ecosystem.

## Status
**Status:** `development`  
**Version:** `0.1.0`

## Architecture
Components include the system services, ShivaCore integration modules and SDK. Data flow and dependencies are defined by the workspace manifests and canonical architecture documentation.

## Features
- Identity and wallet integration
- System services and media
- ShivaCore integration
- Application SDK

## Repository Structure
```text
├── modules
├── sdk
├── docs
└── .github
```

## Requirements
Rust stable and the repository's workspace toolchain are required.

## Installation
```bash
git clone https://github.com/A-TownChain-Okosystems/atc-shivacore.git
cd globus-os
cargo check --workspace
```

## Configuration
Configuration is defined by the workspace manifests and system-specific configuration files.

## Usage
Build and test the workspace with Cargo commands documented by the repository workflows.

## Development
Use the repository workflow and ATC engineering standards for changes.

## Testing
```bash
cargo test --workspace --all-targets
```
Expected result: all applicable workspace tests pass.

## Security
Security issues must not be disclosed publicly; use the repository's official security reporting process and ATC-STD-203.

## Documentation
Canonical documentation is maintained in `docs/` and the repository's governance files.

## Governance
Changes follow ATC governance, evidence and review requirements.

## Standards & Compliance
Applicable standards include ATC-STD-000, ATC-STD-201, ATC-STD-202 and ATC-STD-203.

## Roadmap
See `ROADMAP.md` for the canonical development roadmap.

## Contributing
Contributions must pass the applicable CI and governance gates.

## License
Apache-2.0.

## Maintainers
A-TownChain-Okosystems / ShivaCoreDev.

## Repository Metadata
Canonical repository: `A-TownChain-Okosystems/globus-os`.
