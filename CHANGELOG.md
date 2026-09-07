# Changelog — atc-shivacore

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
