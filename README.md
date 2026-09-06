# ShivaCore — Capability-Microkernel

> **Kernel-Repo des A-TownChain-Ökosystems** · [Monorepo](https://github.com/A-TownChain-Okosystems/a-townchain-os) · [Docs-Hub](https://github.com/A-TownChain-Okosystems/a-townchain-os-docs) · AD-012/AD-013 verbindlich

ShivaCore ist der Capability-Microkernel von **GlobusOS** (siehe
[`globus-os`](https://github.com/A-TownChain-Okosystems/globus-os)). Gemäß AD-012
enthält der Kernel ausschließlich Primitive: Scheduler (DA-HEFT), Memory-Manager,
IPC, Capability-Security, DID/Remote-Caps, Security-Audit — Krypto, Blockchain,
AI, ATCLang, GUI gehören in den Service Space (Migration als eigener Sprint).

## Bestand

| Modul | Inhalt |
|---|---|
| `modules/atc-shivacore` | Kernel-Crate (`kernel/src/`, 60 .rs-Dateien): Capability-System, Prozessverwaltung, Scheduler, IPC, Memory-Manager, ATCFS, ATCNet, DID/Remote-Caps, Knowledge Graph, Consensus/Blockchain/Gossip-Bridge (noch im Kernel — Service-Space-Migration offen), Security Audit. 674/674 Tests, Stable-Rust, Chain-ID 658467 |
| `modules/atc-shivacore-tools` | Kernel-Werkzeuge |

## Architektur-Gates (verbindlich)

- **AD-012** — ShivaCore-Microkernel-Architektur (Hub: `docs/architecture/`)
- **AD-013** — Architektur-Gate v1.1 (SC-ARCH-001…010, SC-001…SC-013)
- **AD-015** — Kernel-Repo-Trennung (06.09.2026, dieses Repo)

## Regeln

- Kernel-Entwicklung läuft hier; Integration (Unified Cargo Workspace, 731/731 Workspace-Tests) bleibt im Monorepo
- Dokumentation kanonisch im Docs-Hub (`SHIVACORE_KERNEL_STATUS.md`, `SHIVACORE_KERNEL_ARCHITECTURE.md`)
- Lizenz: All Rights Reserved (Michael Wroblewski / ShivaCore / A-TownChain-Okosystems)

*Eingerichtet am 06.09.2026 durch Agent Aurora (Base44) im Auftrag des Owners.*
