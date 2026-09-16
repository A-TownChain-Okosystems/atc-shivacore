---
document_id: ATC-AUDIT-SHIVA-20260916
title: Repository Audit — atc-shivacore
version: 1.0.0
status: active
owner: A-TownChain-Okosystems
audit_date: 2026-09-16
standard: ATC-STD-AUDIT-001
---

# Repository Audit — atc-shivacore

## Audit mode

CI-independent source/repository audit performed against the current `main` state and current repository tree. Findings are closed only after source change, re-read, and executable verification where applicable.

## Findings

### F-20260916-SHIVA-001 — P1 — Canonical-source metadata contradiction

**Class:** P1  
**Category:** governance / architecture / consistency  
**Family:** kernel / canonical-source / repository-metadata  
**Tags:** P1, shivacore, canonical-source, governance, metadata, consistency

`.atc/repository.yaml` declared `CORE`, `R4`, `production` and `canonical kernel`, while `STATUS.md` and the repository README state that the active canonical kernel source was migrated to `globus-os/modules/atc-shivacore/kernel/` and this repository is supporting material.

**Correction:** classify the repository as supporting infrastructure (`INFRA`, `R3`, `development`) and set `canonical: false` for the kernel capability.

### F-20260916-SHIVA-002 — P1 — Stale evidence binding

**Class:** P1  
**Category:** evidence / traceability  
**Family:** governance / audit / release-evidence  
**Tags:** P1, evidence, stale, bound-commit, shivacore, traceability

`.atc/evidence/evidence.yaml` was bound to an earlier kernel-source commit while the active kernel source had already moved to `globus-os`. The evidence also claimed the repository itself was canonical.

**Correction:** evidence now explicitly remains unbound until verification CI produces current evidence, and the canonical role is false.

### F-20260916-SHIVA-003 — P1 — Invalid local build instructions

**Class:** P1  
**Category:** correctness / documentation / build  
**Family:** repository-architecture / build-system  
**Tags:** P1, cargo, documentation-drift, build, canonical-source

The root README instructed `cargo build --workspace`, but the repository has no root `Cargo.toml`. The active kernel build is owned by `globus-os`.

**Correction:** README now directs canonical kernel verification to the `globus-os` workspace and removes the false root build contract.

## Security and malware review

Static searches did not establish a malicious payload, credential exposure or unsafe workflow pattern in the inspected repository sources. CI workflows include CodeQL, dependency review, RustSec auditing and determinism checks. These controls reduce specific attack surfaces but do not mathematically prove that the repository is free of all compromise or malware.

For stronger evidence, the ecosystem still needs commit/signature verification, dependency pinning and SBOM/provenance validation, independent security review, reproducible-build comparison, secret scanning and runtime/hardware-specific testing.

## Language and file-format assessment

Rust remains the correct canonical language for the low-level kernel implementation, while this repository can use Markdown/YAML for contracts and governance and Rust/Python only for supporting tooling. No blanket language migration is justified.

## Verification state

**Repository status: IN PROGRESS.**

The canonical-source contradiction and stale evidence are corrected on the audit branch and re-read. Final completion still requires CI verification of the branch and confirmation that no remaining repository-standard gate fails.
