---
document_id: ATC-DOC-SHIVACORE-STATUS-001
title: Repository Status - atc-shivacore
version: 2.2.0
status: active
standard: ATC-STD-MD-001
created: 2026-09-08
updated: 2026-09-16
---

## Status — atc-shivacore

## Property-Value Table

| Property | Value |
|---|---|
| Repository | atc-shivacore |
| Version | 0.1.0 |
| Lifecycle | development / migrated |
| Kernel model | capability-based Rust/no_std microkernel |
| Reuse target | OS-neutral kernel contract |
| Canonical source | `A-TownChain-Okosystems/globus-os/modules/atc-shivacore/kernel/` |
| Kernel CI owner | `A-TownChain-Okosystems/globus-os` |
| Build evidence | Must be taken from the current GlobusOS CI run |
| Boot evidence | Must be taken from the current GlobusOS CI run |
| Production status | NOT_READY |
| Security class | S4 / S-Klasse |
| Documentation | ATC-STD-README-001 / ATC-STD-MD-001 |

## Source-of-truth boundary

The reusable ShivaCore kernel implementation has been migrated into the GlobusOS repository so that the kernel is built, tested, linted and integrated by the same CI pipeline as the operating system.

The canonical implementation path is:

`globus-os/modules/atc-shivacore/kernel/`

The separate `atc-shivacore` repository is no longer the active kernel source tree. Historical copies under `a-townchain-os-docs/docs/archive/` are reference material only and must not be treated as implementation sources.

## Current implementation blocker

A P1 blocker remains in the canonical GlobusOS source: `modules/atc-shivacore/kernel/src/lkm.rs` contains a placeholder `DependencyGraph::dependencies()` whose declared `&[String]` return type cannot be backed directly by the graph's `BTreeSet<String>`. The existing `get_dependencies()` method provides the deterministic owned-vector representation.

This blocker is tracked in GlobusOS issue #18 with classification:

- Class: P1 implementation blocker
- Category: correctness / completeness
- Family: kernel / loadable-kernel-modules / dependency-resolution
- Tags: P1, stub, kernel, lkm, correctness, completeness, api

It must not be represented as production-ready functionality until the API is replaced with a lifetime-safe implementation and verified by GlobusOS CI.

## Evidence policy

Historical test counts and past audit scores are not permanent state. Current GlobusOS CI is authoritative for build/test/lint claims for the canonical kernel source.

## Reuse readiness

The reusable kernel contract remains documented in:

- `docs/specs/SHIVA-KERNEL-REUSE-001.md`
- `docs/specs/SHIVA-HAL-001.md`
- `docs/specs/SHIVA-ABI-001.md`
- `docs/specs/SHIVA-BOOT-001.md`

These specifications define the architecture boundary. They do not by themselves prove production readiness.

## Release gate

`PRODUCTION_READY` requires successful target builds, boot/smoke evidence, IPC/ABI validation, security review, resolution of active implementation blockers and governance approval for the release candidate.
