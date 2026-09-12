# ATC ShivaCore

> **ATC COMPLIANCE: R4 · Standard ATC-STD-201 v1.0.0 · GATE: AUDITED (09.09.2026, Score 94/100) · README: ATC-STD-README-001 CONFORM**

> Capability-basierter Rust Microkernel (L1) für Globus OS im A-TownChain-Ökosystem

**Project:** atc-shivacore
**Organization:** A-TownChain-Okosystems
**Status:** `development`
**Version:** `0.1.0`
**License:** `Apache-2.0`

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

## Overview

ShivaCore ist ein capability-basierter Microkernel in Rust (Layer L1) für das A-TownChain-Ökosystem. Er bildet das funktionale Fundament von Globus OS und stellt sichere Isolation, Capability Guarding, Scheduling und P2P-Kommunikation bereit. Der Kernel wurde im Rahmen der Vault-Restauration (07.09.2026, AD-020/026/027) aus dem Wiki-Vault wiederhergestellt und verifiziert. Wartungszyklus 1 nach ATC-STD-REPO-MAINT-001 wurde am 2026-09-08 erfolgreich abgeschlossen.

## Purpose

ShivaCore stellt die kanonische Implementierung des Microkernel-Kerns (AD-012) im A-TownChain-Ökosystem bereit. Er löst folgende Kernaufgaben:
- Sichere Isolation von Kernel- und Service-Space (AD-028).
- Capability-basierte Rechteverwaltung (CSpace) für Ressourcen.
- Deterministic Scheduling mittels DA-HEFT Algorithmen.
- Sichere P2P-Netzwerkkommunikation (ATC-PROTO-P2P-001).
- Hardware-Abstraktionsschicht (HAL) für Globus OS.

## Scope

- **In Scope:** Microkernel L1, CSpace Capability Management, DA-HEFT Scheduler, IPC, P2P Secure Protocol (K14/K15), Hardware Abstraction Layer, Kernel Boot L0-L10.
- **Out of Scope:** Userspace-Anwendungen (Globus OS Userspace liegt in `globus-os`), AI-Dienste (`aurora-ai`), GameFi-Engines (`genesis-engine`).

## Status

**Status:** `development` — Microkernel ist funktionsfähig. 423/423 Kernel- & P2P-Tests sowie 280 Service-Space-Tests sind grün (Rust 1.98.1). Wartungszyklus 1 (ATC-STD-REPO-MAINT-001) wurde am 08.09.2026 durchgeführt. Chain-ID 658467 verifiziert.

## Architecture

### Components
- **CSpace (Capability Space):** Objektorientierte Rechteverwaltung und Schutzgrenzen.
- **Scheduler:** DA-HEFT (Directed Acyclic Graph Heterogeneous Earliest Finish Time) Scheduling Engine.
- **Memory & IPC:** Microkernel Memory-Management und synchrone/asynchrone IPC.
- **HAL & Network:** Kernel-Primitive (`net.rs`) und sicheres P2P-Protokoll (`p2p_secure.rs`).

### Data Flow
1. Boot-Phase L0-L10 initialisiert Hardware, CSpace und Memory-Manager.
2. Kernel startet Service-Space in isolierten Schutzdomänen.
3. IPC-Nachrichten werden über CSpace-Capability-Guards gefiltert und zugestellt.
4. P2P-Nachrichten verhandeln Handshake (Phasen 10..13) und laufen über TokenBucket Rate-Limiter.

### Dependencies
| Component | Purpose | Required |
|---|---|---|
| Rust 1.98.1 | Core Toolchain & Compiler | Yes |
| ed25519-dalek | Kryptografische Signaturen | Yes |
| spin | Kernel Spinlocks | Yes |

## Features

- **Capability-basiertes Rechtekonzept:** Minimale Rechtevergabe für alle Kernel-Objekte.
- **K14/K15 P2P Secure Protocol:** Handshake, Nonce Anti-Replay, TokenBucket Rate-Limiting.
- **Boot L0-L10 Verifikation:** K29-Kernelstand mit M2-Lauffähigkeitsgate.
- **Umfassende Testabdeckung:** 423 Kernel/P2P-Unit-Tests + 280 Service-Tests.

## Repository Structure

```text
.
├── .atc/                # ATC-Repository-Metadaten
├── .github/             # GitHub Workflows & Dependabot
├── docs/                # Dokumentation & Standards
├── modules/             # Kernel & Tool-Module
├── AGENT_MANIFEST.md    # Agent Manifest
├── AGENTS.md            # AI Agent Instructions
├── ARCHITECTURE.md      # Kernel Architektur-Spezifikation
├── CHANGELOG.md         # Änderungshistorie
├── CODE_OF_CONDUCT.md   # Verhaltenskodex
├── CODEOWNERS           # Repository-Eigentümer
├── CONTRIBUTING.md      # Beitragsrichtlinien
├── GOVERNANCE.md        # Governance-Regeln
├── LICENSE              # Apache-2.0 Lizenz
├── README.md            # Repository Einstiegspunkt
├── ROADMAP.md           # Entwicklungs-Roadmap
├── SECURITY.md          # Sicherheitsrichtlinie
└── STATUS.md            # Maschinenlesbarer Status
```

## Requirements

- Rust 1.98.1 oder neuer (mit `x86_64-unknown-none` bzw. `aarch64-unknown-none` Targets)
- Cargo & Build-Essential Tools
- Python 3.10+ für Workspace-Hilfsskripte

## Installation

```bash
git clone https://github.com/A-TownChain-Okosystems/atc-shivacore.git
cd atc-shivacore
cargo build --workspace
```

## Configuration

Die Repository-Konfiguration liegt unter `.atc/repository.yaml`. Kernel-Parameter können über Cargo-Features angepasst werden.

## Usage

Starten der Boot-Sequenz im Simulator / Target:

```bash
cargo run --bin boot --manifest-path modules/atc-shivacore/boot/Cargo.toml
```

## Development

- Beachten Sie die A-TownChain Development Rules (ATC-STD-000, ATC-STD-201).
- Verwenden Sie Conventional Commits.
- Integration in das Monorepo erfolgt über `scripts/sync_modules.py` in `a-townchain-os`.

## Testing

Ausführen der gesamten Testsuite:

```bash
cargo test --workspace
```

**Erwartetes Ergebnis:** `423/423 PASS` (394 Bestandstests + 29 P2P-Tests).

## Security

Security issues **must not** be disclosed publicly. Report vulnerabilities through the official ATC security reporting process or contact `security@a-townchain.org` (ATC-STD-203, SECURITY.md).

## Documentation

- Full Architecture Spec: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Development Roadmap: [`ROADMAP.md`](ROADMAP.md)
- Organizational Docs: [a-townchain-os-docs](https://github.com/A-TownChain-Okosystems/a-townchain-os-docs)

## Governance

This repository is governed according to ATC-STD-000 v1.3.0 (A-TownChain Enterprise Governance Framework). Architecture changes require approval via SCR (System Change Request).

## Standards & Compliance

| Standard | Version | Compliance |
|---|---:|---|
| ATC-STD-000 | 1.3.0 | ✅ APPROVED |
| ATC-STD-README-001 | 1.0.0 | ✅ APPROVED |
| ATC-STD-MD-001 | 1.0.0 | ✅ APPROVED |
| ATC-STD-201 | 1.0.0 | ✅ APPROVED |
| ATC-STD-202 | 1.1.0 | ✅ APPROVED |
| ATC-STD-203 | 1.0.0 | ✅ APPROVED |
| ATC-PROTO-P2P-001 | 1.0.0 | ✅ APPROVED |
| ATC-STD-REPO-MAINT-001 | 1.0.0 | ✅ APPROVED |

## Roadmap

Die kanonische Roadmap ist in [`ROADMAP.md`](ROADMAP.md) dokumentiert und wird über das ATC Development Management (LAUFFAEHIGKEITS_ROADMAP M1-M8) nachverfolgt.

## Contributing

Beiträge sind willkommen! Siehe [`CONTRIBUTING.md`](CONTRIBUTING.md) für Richtlinien.

## License

Standardisiert unter **Apache-2.0** (siehe [`LICENSE`](LICENSE)).

## Maintainers

- **Organization:** A-TownChain-Okosystems
- **Owner:** Michael Wroblewski (GitHub: ShivaCoreDev)
- **Maintainer:** ShivaCore Core Team / Aurora Superagent

## Changelog

Änderungen sind in [`CHANGELOG.md`](CHANGELOG.md) protokolliert.
