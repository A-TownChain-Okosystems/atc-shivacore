# atc-shivacore [L1]

ShivaCore Microkernel — Rust, Capability-basiert, K29-Stand (AD-012/013).

**Vault-Restauration (07.09.2026, AD-020/026/027):** Inhalt aus dem Wiki-Vault
(docs/archive/monorepo-full/) restauriert — vor der Repo-Leerung byte-identisch gesichert. Chain-ID 658467 bereits im Vault-Stand (0 Alt-Reste, verifiziert).

**Module:** atc-shivacore, atc-shivacore-tools

**Meile (AD-027):** M2 — Kernel laeuft: cargo test 674/674 + KernelState::boot() L0-L10 (Test-Verifikation im Rebuild-Lauf)

**Hinweis:** Basis fuer den Rebuild; Gate-Kriterien laut LAUFFAEHIGKEITS_ROADMAP
(a-townchain-os-docs/docs/roadmap/).

---

## ATC Compliance & Governance (ATC-STD-201 / 202 / 203)

**ATC COMPLIANCE: R4** — auditiert am 2026-09-07 (atc-repo-audit; R-Level aus `.atc/repository.yaml`).
Architekturentscheidungen: zentral im [DECISIONS_REGISTER](https://github.com/A-TownChain-Okosystems/a-townchain-os-docs/blob/main/docs/DECISIONS_REGISTER.md) (AD-Nummern verbindlich; lokale Entscheidungen in `docs/decisions/`).

- **Purpose:** ShivaCore Microkernel (AD-012) — das Fundament von Globus OS.
- **Scope:** Layer L1, Domain kernel — atc-shivacore als CORE in der 23-Repo-Landschaft (AD-024/026).
- **Architecture:** Capability-Microkernel: CSpace, Scheduler, Memory, IPC, HAL; Service-Space strikt getrennt (AD-028); net.rs als K12-HAL-Primitive.
- **Features:** 674/674 Tests (394 Kernel + 280 Service, Rust 1.98.1); Boot L0-L10; Chain-ID 658467; DA-HEFT-Scheduling-Forschung.
- **Installation:** Modul-Build je Sprache (rust); Integration via Monorepo-Workspace (a-townchain-os, sync_modules.py).
- **Development:** Conventional Commits; Governance-Regeln aus atc-standards; Naming gemaess ATC-STD-000 §7.
- **Testing:** cargo test --workspace: 674/674; Boot-Chain L0-L10 verifiziert (M2-Gate erfuellt).
- **Security:** SECURITY.md; S-Klasse S4; ATC-STD-203 Release-Gates; Emergency-Prozess ATC-STD-000 §32.
- **Roadmap:** Einordnung in die Lauffaehigkeits-Roadmap M1-M8 (AD-027) und Bauhierarchie L0-L7 (AD-026).
- **Version:** CHANGELOG.md; SemVer; Releases als ATC-REL-X.Y.Z.
- **License:** Proprietaer — All Rights Reserved, Michael Wroblewski / ShivaCore / A-TownChain-Okosystems (ATC-LIC/ATS-LIC).

**Maintenance (ATC-STD-REPO-MAINT-001):** Zyklus 1 am 2026-09-08 — Status-Label **MAINTENANCE_REQUIRED**
(P1 offen: Build/Test-CI, Major-Dependency-REVIEW; P2: Tag/Release HELD). Report: RUN-001 im Docs-Hub.
