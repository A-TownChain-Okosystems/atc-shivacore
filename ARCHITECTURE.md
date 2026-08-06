# 🌳 Architektur — atc-shivacore

> **Stand:** 2026-08-06 | **Commit:** 1750188
> **Teil von:** [A-TownChain Ökosystem](https://github.com/A-TownChain-Okosystems)

## Statistik

| Metrik | Wert |
|--------|------|
| Dateien | 66 |
| Zeilen | 50,580 |
| .atc | 0 |
| .py | 0 |
| .rs | 53 |
| .ts/.tsx | 0 |
| .md | 5 |

## Verzeichnisstruktur

```
├── boot/ (2 files, 38 lines)
│   ├── src/ (1 files, 30 lines)
│   │   └── main.rs (30 lines)
│   └── Cargo.toml (8 lines)
├── kernel/ (56 files, 47,234 lines)
│   ├── .cargo/ (1 files, 2 lines)
│   │   └── config.toml (2 lines)
│   ├── src/ (52 files, 47,194 lines)
│   │   ├── ai.rs (75 lines)
│   │   ├── allocator.rs (46 lines)
│   │   ├── atcfs.rs (627 lines)
│   │   ├── atcnet.rs (1139 lines)
│   │   ├── ats1000.rs (85 lines)
│   │   ├── block.rs (548 lines)
│   │   ├── capability.rs (248 lines)
│   │   ├── consensus.rs (961 lines)
│   │   ├── container.rs (2757 lines)
│   │   ├── container_net.rs (632 lines)
│   │   ├── contract.rs (38 lines)
│   │   ├── cow.rs (1484 lines)
│   │   ├── cross_subsystem.rs (483 lines)
│   │   ├── devfs.rs (921 lines)
│   │   ├── did.rs (350 lines)
│   │   ├── elf_loader.rs (1104 lines)
│   │   ├── framebuffer.rs (122 lines)
│   │   ├── fs_journal.rs (1161 lines)
│   │   ├── gdt.rs (59 lines)
│   │   ├── genesis.rs (1111 lines)
│   │   ├── genesis_bridge.rs (1097 lines)
│   │   ├── gossip_bridge.rs (1410 lines)
│   │   ├── interrupts.rs (100 lines)
│   │   ├── kernel_init.rs (431 lines)
│   │   ├── knowledge_graph.rs (755 lines)
│   │   ├── lib.rs (73 lines)
│   │   ├── lkm.rs (2998 lines)
│   │   ├── main.rs (100 lines)
│   │   ├── mempool.rs (75 lines)
│   │   ├── module_security.rs (1682 lines)
│   │   ├── net.rs (802 lines)
│   │   ├── p2p.rs (861 lines)
│   │   ├── page_fault.rs (1371 lines)
│   │   ├── power.rs (1153 lines)
│   │   ├── process.rs (360 lines)
│   │   ├── remote_caps.rs (629 lines)
│   │   ├── security.rs (879 lines)
│   │   ├── security_audit.rs (1264 lines)
│   │   ├── serial.rs (42 lines)
│   │   └── signals.rs (2249 lines)
│   ├── .gitignore
│   ├── Cargo.lock
│   └── Cargo.toml (38 lines)
├── .gitignore
├── CHANGELOG.md (21 lines)
├── Cargo.toml (12 lines)
├── FILE_REGISTER.md (2161 lines)
├── LICENSE
├── README.md (1074 lines)
├── ROADMAP.md (21 lines)
└── STATUS.md (19 lines)
```

---
*Auto-generiert 2026-08-06 · Aurora (MasterBrain · Base44)*
