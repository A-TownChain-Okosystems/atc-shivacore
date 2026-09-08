---
document_id: ATC-DOC-SHIVACORE-ARCH-001
title: Microkernel Architecture Specification - atc-shivacore
version: 1.0.0
status: active
standard: ATC-STD-MD-001
created: 2026-09-08
updated: 2026-09-08
---

# Architecture — atc-shivacore

## Systemübersicht

`atc-shivacore` ist der capability-basierte Rust-Microkernel (Layer L1) des A-TownChain-Ökosystems. Er bildet das Fundament für Globus OS.

## Kernkomponenten

### CSpace (Capability Space)
- Objektorientierte Rechteverwaltung für Systemressourcen.
- Striktes Capability Guarding zur Verhinderung unautorisierter Speicher- und Hardwarezugriffe.

### Scheduler
- DA-HEFT (Directed Acyclic Graph Heterogeneous Earliest Finish Time) Scheduling-Forschung und Determinismus.
- Thread- und Process-Management mit minimaler Latenz.

### Memory Management
- Paging, Frame Allocation, Isolation zwischen Kernel- und Service-Space.

### IPC & P2P
- Synchrone und asynchrone Message-Passing-Interfaces.
- P2P Secure Layer (`p2p_secure.rs`) gemäß ATC-PROTO-P2P-001 mit Handshake, Anti-Replay und Rate-Limiting.

## Diagramm / Datenfluss

```text
+-------------------------------------------------------+
|                    Service Space                      |
+-------------------------------------------------------+
                           | IPC (Capability-guarded)
+--------------------------v----------------------------+
|                  ShivaCore Microkernel                |
|  +----------------+  +--------------+  +-----------+  |
|  | CSpace & Guard |  | DA-HEFT Sch. |  | P2P Sec.  |  |
|  +----------------+  +--------------+  +-----------+  |
+-------------------------------------------------------+
                           | Hardware Abstraction
+--------------------------v----------------------------+
|                        Hardware                       |
+-------------------------------------------------------+
```
