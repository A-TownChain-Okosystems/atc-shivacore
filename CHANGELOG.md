# Changelog — atc-shivacore

## [0.1.0] — 2026-09-08 (Maintenance-Zyklus 1, ATC-STD-REPO-MAINT-001)
- **Version-Baseline 0.1.0** für kernel/boot/service_space vereinheitlicht (vorher 0.0.1).
- **Lizenz-Metadatum korrigiert:** `.atc/repository.yaml` license.type proprietary -> Apache-2.0 (SPDX) — Angleichung an LICENSE-Datei (SCR-0036, F-046).
- **Maintenance-Befunde:** 8 Dependabot-Major-PRs (rand/ed25519-dalek/pc-keyboard/spin/uart_16550) im REVIEW-Pfad; Build/Test-CI fehlt (P1, Owner-Aktion GH013); Tag/Release v0.1.0 HELD bis Build-Gate; 19 TODO/FIXME inventarisiert (Report RUN-001).
- Report: a-townchain-os-docs/docs/maintenance/2026-09-08_atc-shivacore_MAINT-RUN-001.md

## [Unreleased] — 2026-09-08
- **K14-Upgrade: P2P v1.0.0 (ATC-PROTO-P2P-001 implementiert, SCR-0028).**
  NEU kernel/src/p2p_secure.rs (~1250 Zeilen inkl. 29 Unit-Tests):
  - Envelope 9+1 Pflichtfelder mit kanonischer Byte-Serialisierung
    (§3.1) und Signatur-Grandungslage Domain-Separation `ATC-P2P-v1\0`
  - Message-Types 10..13 fuer den 6-Phasen-Handshake (§8):
    CapabilityExchange, AuthChallenge, AuthResponse, KeyExchange —
    mit Version-Verhandlung (0.9/1.0.0) und Capability-Bitmap
  - Peer-Lifecycle: Verified nach Auth-Phase, Banned mit 24h-Fenster (§5/§10)
  - Replay-Schutz: Nonce-Einmaligkeit je Absender, Message-ID-Seen-Set
    (bounded 4096, FIFO), Timestamp-Fenster ±120s (§15)
  - Rate-Limiting ueber K15 TokenBucket: 100 msgs/s + 256 kiB/s je Peer (§14)
  - Fehlerkatalog ATC-PROTO-P2P-001..019 mit Kategorie/Retryable/Severity (§12)
  - Kryptografie-Abstraction-Layer (SignatureProvider-Trait,
    PROTOCOL-001 §12): SimulatedSigner-Backend, Ed25519-Backend ohne
    Protokoll-Codeaenderung austauschbar; symmetrische Session-Key-Ableitung
  - v0.9-Kompatibilitaetsmodus (§3.2): K14-Wire-Format bleibt lesbar,
    Chain-ID-Abweisung inklusive im Dual-Mode-Empfang
  - Regression: K14-Basis (p2p.rs) unveraendert, alle 394 Bestandstests gruen
  - Kernel gesamt: 423/423 Tests gruen (394 Bestand + 29 neu)

## [Unreleased] — 2026-09-07
- Governance-Ueberarbeitung nach ATC-STD-201/202/203: .atc-Metadaten
  (repository/ownership/lifecycle/compliance.yaml), SECURITY.md, CODEOWNERS,
  docs/REPOSITORY_STANDARD.md, Governance-CI (governance-ci.yml),
  ATC-COMPLIANCE-Anhang im README. R-Level: R4.
