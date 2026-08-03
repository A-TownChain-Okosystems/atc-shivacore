# ShivaCore — Bare-Metal Kernel (K-Sprint 0 ✅)

Eigener Kernel von Grund auf in Rust (`no_std`, `x86_64-unknown-none`) — kein
Linux-Unterbau, kein Fremdcode ausser dem minimalen Boot-Protokoll
(`bootloader` 0.11 Crate). Teil des GlobusOS-Betriebssystems
(GlobusOS = OS gesamt, ShivaCore = nur der Kernel darin).

## Status: K-Sprint 0 abgeschlossen (07.07.2026)

Verifiziert per QEMU-Boot-Test (BIOS-Image):
- ✅ Kernel-ELF wird vom Bootloader korrekt geladen und Entry-Point erreicht
- ✅ Serielle Debug-Konsole funktioniert (`serial_println!` via UART 16550)
- ✅ Framebuffer-Textausgabe funktioniert (Pixel-Modus, nicht klassisches VGA-Text)
- ✅ Kein Hang, sauberer Übergang in Idle-Loop (`hlt`)

**Vorheriges Problem (05.-06.07.) war ein Diagnose-Irrtum, kein echter Bug:**
Frühere Sessions vermuteten einen Hang zwischen Bootloader und Kernel wegen
leerem Serial-Log. Root-Cause-Analyse (07.07.) mit einem minimalen
Raw-Serial-Diagnose-Kernel (direkter Port-Write vor jeglicher Initialisierung)
zeigte: der Kernel wurde immer schon erreicht und konnte schreiben. Der
vollständige Kernel (inkl. `lazy_static`-Serial-Init und Framebuffer-Init)
bootet bei erneutem Test einwandfrei durch — vermutlich war das leere Log in
früheren Sessions ein Artefakt eines fehlgeschlagenen/falsch konfigurierten
Testlaufs, kein Kernel-Bug.

## Bauen

```bash
cd kernel && cargo build --release
cd ../boot && cargo run --release -- \
  ../kernel/target/x86_64-unknown-none/release/shivacore \
  ../images
```

Erzeugt `images/shivacore-bios.img` und `images/shivacore-uefi.img`.

## Testen (QEMU)

```bash
qemu-system-x86_64 -drive format=raw,file=images/shivacore-bios.img \
  -serial stdio -display none -no-reboot
```

Erwartete Ausgabe:
```
ShivaCore: Kernel-Einstiegspunkt erreicht.
ShivaCore: Framebuffer-Ausgabe erfolgreich.
ShivaCore: Boot vollstaendig. Uebergabe an Idle-Loop.
```

## Nächster Schritt: K-Sprint 1

CPU-Grundlagen — GDT, IDT, Interrupt-Handler (Breakpoint, Double-Fault, Page-Fault),
PIC-Remapping. Baut direkt auf diesem Kernel auf (`kernel/src/main.rs`).

## Struktur

- `kernel/` — der eigentliche bare-metal Kernel-Crate (kompiliert zu ELF, läuft ring 0)
- `boot/` — Host-Tool, das aus dem Kernel-ELF bootfähige BIOS/UEFI-Images baut
  (nutzt `bootloader::BiosBoot`/`UefiBoot`, läuft NICHT im Kernel-Kontext)


## Status: K-Sprint 1 abgeschlossen (07.07.2026)

GDT + TSS (dedizierter Double-Fault-Stack via IST), IDT mit Breakpoint-/
Double-Fault-/Page-Fault-Handlern, PIC-Remapping (8259 von 0x08-0x0F auf
0x20-0x2F), Timer- + Keyboard-Interrupts aktiv. QEMU-verifiziert: Breakpoint
(`int3`) kehrt sauber zurueck, kein Crash, Idle-Loop laeuft weiter.

**Gefixter Bug:** Erster Testlauf loeste einen Double Fault direkt nach dem
Breakpoint-Handler aus. Ursache: nach dem Laden des eigenen (minimalen) GDT
zeigte der alte Stack-Segment-Selektor (SS, vom Bootloader-GDT) ins Leere.
Bei der IRETQ-Rueckkehr aus dem Interrupt wird SS zwingend neu geladen und
validiert -> #GP waehrend IRETQ -> vom Prozessor als Double Fault eskaliert.
Fix: SS nach dem GDT-Laden explizit auf den Null-Selektor setzen (in
Long-Mode bei CPL0 fuer das Stack-Segment zulaessig, da Flat-Memory-Modell).

## Nächster Schritt: K-Sprint 2

Speicherverwaltung — Paging (aktuelle Page-Tables auslesen/verstehen), Heap-
Allokator (`#[global_allocator]`), damit `alloc`/`Box`/`Vec` nutzbar werden.


## Status: K-Sprint 2 abgeschlossen (07.07.2026)

Paging-Mapper (`OffsetPageTable` über das vom Bootloader linear gemappte
physische RAM), einfacher `BootInfoFrameAllocator` (iteriert die vom
Bootloader gemeldete `MemoryRegions`-Karte nach freien 4-KiB-Frames), Heap
(100 KiB, `linked_list_allocator`). `alloc` (Box/Vec/String) ist jetzt im
Kernel nutzbar. QEMU-verifiziert: `Box::new(41)` und `Vec` mit Summe 0..10=45
funktionieren fehlerfrei, kein Crash.

Voraussetzung war eine `BootloaderConfig` mit `mappings.physical_memory =
Some(Mapping::Dynamic)`, eingebettet via `entry_point!(kernel_main, config =
&BOOTLOADER_CONFIG)` in `main.rs`.

## Nächster Schritt: K-Sprint 3

Multitasking — Prozess-/Task-Struktur, einfacher Scheduler, Context-Switch.


## Ehrlicher Pitch — was ShivaCore ist (und was nicht)

**Nicht:** "Schon jetzt besser als Linux/seL4/Redox OS." ShivaCore ist Stand
07.07.2026 drei K-Sprints alt (Boot, GDT/IDT/PIC, Paging/Heap) — kein
fairer Vergleich zu Kerneln mit jahrzehntelanger Härtung.

**Was heute real ist:** Rust `no_std` statt C ist der einzige bereits
beweisbare Vorteil — ganze Bug-Klassen (Buffer-Overflow, Use-after-free,
Data Races), die bei Linux-Kernel-Treibern die häufigste CVE-Quelle sind,
sind durch den Rust-Borrow-Checker strukturell ausgeschlossen.

**Was der Plan ist (ATC-1 bis ATC-40, noch nicht implementiert):** Die
meisten "Blockchain-OS"-Projekte sind in Wahrheit normales Linux mit einem
Blockchain-Node als Userspace-App obendrauf — architektonisch nichts Neues.
Der Anspruch hinter GlobusOS/ShivaCore ist, Dinge wie P2P-Konsens,
DID-/Agent-Identität und Capability-basierte Security näher an den Kernel
zu bringen statt als nachträglich aufgesetzten Daemon.

**Was heute fehlt:** Dateisystem, Netzwerk-Treiber, Multitasking (in
Arbeit, K-Sprint 3), Hardware-Breite, Jahre an Battle-Testing.

**Kurz:** kleine, auditierbare, speichersichere Codebasis mit
blockchain-nativer Architektur als Ziel — nicht "schon jetzt überlegen".
Für alle Agenten, die an diesem Projekt arbeiten: bitte diese Erwartungshaltung
in Doku/Kommunikation nach außen konsistent halten, keine Überverkaufs-Sprache.

---

## K-Sprint 3a: Capability-System (03.08.2026)

`kernel/src/capability.rs` — Rust-Port des Python-Capability-Systems.

- `Rights` (Bitfield: READ/WRITE/EXEC/DELEGATE)
- `CapabilityTable` (Spinlock-geschuetzt, BTreeMap)
- `create()` / `delegate()` (Attenuation) / `check()` / `revoke()` (kaskadierend)
- 8/8 Tests gruen (`cargo test`)

## K-Sprint 3b: Prozessverwaltung (03.08.2026)

`kernel/src/process.rs` — ProcessManager mit Capability-Integration.

- `ProcessControlBlock` (PID, Typ, Prioritaet, Zustand, Parent/Children)
- `spawn()` — erzeugt Prozess + automatische Memory-Cap (READ/WRITE/EXEC/DELEGATE)
- `spawn_child()` — Kind-Prozess mit Parent-Verknuepfung
- `kill()` — kaskadierender Capability-Widerruf + Zustand→Terminated
- `wait()` — Exit-Code-Abfrage
- Zustandsautomaten: Ready↔Running, →Blocked→Ready (Preemption/Block)
- 10/10 neue Tests + 8/8 Capability-Tests = 18/18 gesamt gruen

## K-Sprint 4: DA-HEFT Scheduler (03.08.2026)

`kernel/src/scheduler.rs` — Deadline-Aware Heterogeneous Earliest-Finish-Time in Rust.

- `Accelerator` Trait — Hardware-Abstraktion (CPU/GPU/NPU/TPU), austauschbar fuer echte Hardware
- `SimulatedAccelerator` — fuer Tests und Software-Validierung
- Upward-Rank (HEFT): iterativ aus Successor-Map berechnet, Entry-Tasks zuerst
- Deadline-Aware: Tasks die ihre Deadline verfehlen werden markiert
- Thermisches Throttling: ueberhitzte Beschleuniger werden uebersprungen
- Speicher-Constraint: Beschleuniger ohne ausreichenden Speicher wird uebersprungen
- 10/10 neue Scheduler-Tests + 18/18 bestehende = 28/28 gesamt gruen

## K-Sprint 5: Inter-Process Communication (03.08.2026)

`kernel/src/ipc.rs` — Channel-basierte IPC mit Capability-Durchsetzung.

- `IpcSubsystem` — verwaltet alle Channels, erzeugt Channel mit auto-Caps
- `create_channel()` — Owner bekommt WRITE+READ+DELEGATE Capabilities automatisch
- `send()` / `recv()` — Capability-gegated (WRITE zum Senden, READ zum Empfangen)
- `grant_access()` — Delegation von Channel-Rechten an andere Prozesse (Attenuation)
- `close_channel()` — schliesst Channel + kaskadierender Capability-Widerruf
- `close_all_for()` — schliesst alle Channels eines Prozesses (fuer kill())
- FIFO-Buffer mit konfigurierbarer Kapazitaet
- 22/22 IPC-Tests (12 Basis + 10 Capability-Gating) + 28/28 bestehende = 50/50 gesamt gruen
- Security-Fix: recv() prueft Capability VOR Buffer-Inspektion (verhindert Info-Lecks)

## K-Sprint 5: Inter-Process Communication (03.08.2026)

`kernel/src/ipc.rs` — Channel-basierte IPC mit Capability-Durchsetzung.

- `IpcSubsystem` — verwaltet alle Channels, erzeugt Channel mit auto-Caps
- `create_channel()` — Owner bekommt WRITE+READ+DELEGATE Capabilities automatisch
- `send()` / `recv()` — Capability-gegated (WRITE zum Senden, READ zum Empfangen)
- `grant_access()` — Delegation von Channel-Rechten an andere Prozesse (Attenuation)
- `close_channel()` — schliesst Channel + kaskadierender Capability-Widerruf
- `close_all_for()` — schliesst alle Channels eines Prozesses (fuer kill())
- FIFO-Buffer mit konfigurierbarer Kapazitaet
- 22/22 IPC-Tests (12 Basis + 10 Capability-Gating) + 28/28 bestehende = 50/50 gesamt gruen
- Security-Fix: recv() prueft Capability VOR Buffer-Inspektion (verhindert Info-Lecks)

## K-Sprint 6: DID + Remote-Capability-Tickets (03.08.2026)

`kernel/src/did.rs` — Dezentrale Knoten-Identitaet:
- `Did` — did:shivacore:<public-key> Format
- `CryptoProvider` Trait — abstrahierte Kryptografie (Ed25519 oder Hardware-Enklave)
- `SoftwareSigner` — deterministischer Software-Signer fuer Tests
- Sign/Verify mit trait-basierter Abstraktion

`kernel/src/remote_caps.rs` — Remote-Capability-Tickets (RCT):
- `RemoteCapabilityTicket` — kryptographisch signierte Delegation
- `issue_ticket()` — Issuer signiert Ticket fuer Subject
- `RemoteCapabilityResolver` — validiert Signatur, Subject, Replay, Deadline, Constraints
- `resolve_chain()` — mehrstufige Delegation mit Attenuation-Pruefung
- `LocalCap` — lokale Capability mit Verbrauchszaehler (max_operations)
- `NonceStore` — Replay-Schutz (BTreeSet)
- 22/22 Tests (6 DID + 16 RCT) + 50/50 bestehende = 72/72 gesamt gruen

## K-Sprint 6b: Ed25519 Signatur-Implementierung (03.08.2026)

`kernel/src/did.rs` erweitert:
- **Ed25519Signer** — echte Ed25519-Signaturen mit `ed25519-dalek` crate
- `did:shivacore:ed25519:<hex-pubkey>` Format (32-byte Public Key = 64 hex chars)
- 64-Byte Ed25519-Signaturen, verifiziert durch `VerifyingKey::verify()`
- `from_seed()` — deterministische Schluesselerzeugung fuer reproduzierbare Tests
- SoftwareSigner bleibt fuer deterministische Logik-Tests (XOR-basiert)
- 9 neue Ed25519-Tests: DID-Format, sign+verify, wrong-signer, tampered-payload,
  tampered-sig, short-sig, deterministic-seed, cross-verify-with-RCT, large-payload
- Gesamt: 81/81 Tests gruen (8 Cap + 10 Proc + 10 Sched + 22 IPC + 15 DID + 16 RCT)

## K-Sprint 7: Knowledge Graph (03.08.2026)

`kernel/src/knowledge_graph.rs` — Nativer Triple-Store fuer strukturiertes Wissen.

- `KnowledgeGraph` — Triple-Store (Subject-Predicate-Object) mit SPO/OSP/PSO Indices
- `Entity` — eindeutige ID, Label, Type, Creator, Triple-Counter
- `ObjectValue` — Entity-Referenz, Integer, String, Bytes, Boolean
- Capability-gated: `create_entity()`, `add_triple()`, `query()`, `remove_triple()`, `delete_entity()` pruefen Caps
- `QueryPattern` — Wildcard-Query (None = Match-All)
- `outgoing()` / `incoming()` — gerichtete Graph-Traversierung
- `transitive_closure()` — Pfadsuche ueber mehrere Hops mit max_depth + Zyklus-Schutz (visited-Set)
- `grant_read()` — Delegation von Lesezugriff an andere Prozesse
- 18/18 neue Tests + 81/81 bestehende = 99/99 gesamt gruen

## K-Sprint 8: Virtual File System (VFS) (03.08.2026)

`kernel/src/vfs.rs` — Capability-gegates VFS mit In-Memory-Backend.

- `Vfs` — zentrale VFS-Instanz, verwaltet Inodes, File-Handles, Pfad-Auflösung
- `Inode` — eindeutige ID, Name, Typ (File/Directory/Symlink), Parent/Children, Daten
- `FileHandle` — File-Deskriptor mit Position, Mode (Read/Write/ReadWrite/Append/Create), PID
- Verzeichnis-Operationen: `mkdir()`, `list_dir()`, `rmdir()` (nur wenn leer, Root geschützt)
- Datei-Operationen: `create_file()`, `open()`, `read()`, `write()`, `close()`, `seek()`
- `OpenMode` — Read, Write, ReadWrite, Append, Create
- Symlink-Unterstützung: `create_symlink()`, `read_symlink()`
- Pfad-Normalisierung: `..` und `.` werden korrekt aufgelöst
- `stat()` — Metadaten (Typ, Größe, Owner, Permissions, Zeitstempel)
- `remove_file()` — schließt offene Handles automatisch vor Löschung
- `tree()` — Debug-Baum-Anzeige des gesamten VFS
- Capability-Gating: alle Operationen prüfen READ/WRITE-Rechte
- 22/22 neue VFS-Tests + 99/99 bestehende = 121/121 gesamt gruen

**Nächster Schritt:** K-Sprint 9 — Device-Driver-Framework oder Netzwerk-Stack.

## K-Sprint 9: Syscall Interface (ATC-96) (03.08.2026)

`kernel/src/syscall.rs` — Einheitlicher Dispatch-Layer fuer alle Kernel-Subsysteme.

- `SyscallDispatcher` — zentrale Dispatch-Funktion, leitet Syscalls an Subsysteme weiter
- 33 Syscalls definiert (ATC-96-konform): Prozess (spawn/kill/wait), VFS (open/read/write/
  close/seek/mkdir/rmdir/listdir/stat/create_file/remove_file/symlink/readlink),
  IPC (create/send/recv/grant/close), Capability (create/delegate/check/revoke),
  Scheduler (yield/info), Knowledge Graph (query/create_entity/add_triple),
  Memory (alloc/free/memcpy)
- `Context` — drei Ausfuehrungs-Contexte: Node (vollzugriff), Contract (nur alloc/free,
  keine I/O), Test (alle mit Mocks) — ATC-96 Abschnitt 3 konform
- Gas-Tracking — jeder Syscall hat definierte Gas-Kosten (ATC-96 Abschnitt 4),
  Dispatcher verbraucht Gas und blockiert bei OutOfGas
- `SyscallArg` — typisierte Argumente (U64, String, Bytes)
- `SyscallResult` — Success(u64), SuccessString, SuccessList, Ok, Error
- `SyscallError` — PermissionDenied, OutOfGas, InvalidArgument, NotFound,
  AlreadyExists, CapabilityDenied, VfsError, ProcessError, IpcError, UnknownSyscall
- Capability-Gating: jeder Syscall prueft READ/WRITE/EXEC/DELEGATE vor Ausfuehrung
- 22/22 neue Syscall-Tests + 121/121 bestehende = 143/143 gesamt gruen

**Architektonische Bedeutung:** Mit K9 sind alle Kernel-Subsysteme (K3-K8) erstmals
ueber eine einheitliche, getestete Schnittstelle erreichbar. Das ist die
Voraussetzung fuer Userspace-Prozesse (Ring-3) und echtes Multitasking — der
Kernel ist jetzt ein echtes OS, das Prozesse bedient, nicht nur eine Sammlung
von Modulen.

**Naechster Schritt:** K-Sprint 10 — Block-Device-Layer (virtio-blk fuer QEMU)
oder Timer/Clock-Subsystem (Praezision fuer Scheduler).


## Kernel-Hilfsmodule (nicht in K-Sprints dokumentiert)

- `kernel/src/serial.rs` — Serielle Debug-Konsole (QEMU: `-serial stdio`), `println!`-Backend
- `kernel/src/framebuffer.rs` — Framebuffer-Textausgabe (gerasterte Glyphen, kein VGA-Text-Modus)
- `kernel/src/ats1000.rs` — ATS-1000 ShivaCore Interface (Traits: ProcessManager, MemoryManager, FileSystem, NetworkStack)
- `kernel/src/remote_caps.rs` — Remote-Capability-Tickets (RCT), kryptografisch signierte Delegation an fremde Knoten

## K-Sprint 10: Timer/Clock-Subsystem (03.08.2026)

`kernel/src/timer.rs` — Monotone Uhr, Deadline-Tracking, Sleep-Queue.

- `TimerSource` Trait — Abstraktion für HPET/PIT/TSC (Hardware) oder Simulation
- `SimulatedTimerSource` — RAM-basierte Zeitquelle für Tests, advance()/set()
- `MonotonicClock` — uptime_ns/ms/secs, uptime_string(), kapselt TimerSource
- `TimerManager` — Sleep-Queue mit Deadline-Sortierung (BTreeMap)
- `TimerCallback` — Wakeup(pid), Periodic(interval_ns), Alarm (one-shot)
- `sleep()` — registriert Prozess-Sleep mit Deadline
- `schedule_periodic()` — periodischer Timer mit automatischem Re-Register
- `schedule_alarm()` — einmaliger Alarm
- `cancel()` — bricht Timer ab
- `tick()` — prüft alle Deadlines, liefert fired events, re-registriert periodische
- `next_deadline()` / `time_to_next_deadline()` — Scheduler-Integration
- `duration` — Hilfsfunktionen (from_ms/secs/us/mins, to_ms/secs/us)
- 20/20 neue Timer-Tests + 143/143 bestehende = 163/163 gesamt gruen

## K-Sprint 11: Block-Device-Layer (03.08.2026)

`kernel/src/block.rs` — Block-Storage-Abstraktion, Cache, MBR-Partitionen.

- `BlockDevice` Trait — read_block/write_block, block_count, capacity, is_read_only
- `SimulatedBlockDevice` — RAM-backed Block-Device für Tests (read-only mode supported)
- `BlockBuffer` — LRU-Block-Cache mit Dirty-Tracking und Flush
  - read() — Cache-Hit/Miss-Statistik, automatische Eviction bei vollem Cache
  - write() — schreibt in Cache, markiert dirty
  - flush() — schreibt alle dirty Blocks auf das Gerät
  - clear() — flush + Cache leeren
- `MBRPartitionTable` — MBR-Parsing (0x55AA-Signatur, 4 Partition-Einträge)
  - PartitionEntry: bootable, type, start_lba, block_count
- 18/18 neue Block-Tests + 163/163 bestehende = 181/181 gesamt gruen

## K-Sprint 12: Netzwerk-Stack Foundation (03.08.2026)

`kernel/src/net.rs` — Ethernet, ARP, NetworkDevice-Abstraktion.

- `MacAddress` — 6-Byte, broadcast/zero, is_broadcast/is_zero, to_string
- `Ipv4Address` — 4-Byte, broadcast/zero, is_broadcast/is_zero, to_string
- `EthernetFrame` — dst/src/ethertype/payload, to_bytes/from_bytes
  - ETH_TYPE_ARP (0x0806), ETH_TYPE_IPV4 (0x0800)
- `ArpPacket` — ARP-Request/Reply, serialize/deserialize (28 Bytes)
  - ARP_HW_ETHERNET, ARP_OP_REQUEST, ARP_OP_REPLY
- `ArpTable` — IP→MAC Mapping mit Timeout und permanenten Einträgen
  - lookup(), insert(), insert_permanent(), remove(), purge_expired()
- `NetworkDevice` Trait — send_frame/recv_frame, mac_address, mtu, is_up, name
- `LoopbackDevice` — RAM-basiertes Netzwerk-Device für Tests (Queue-basiert)
- `NetworkStack` — verbindet Device + ARP
  - arp_request() — sendet ARP-Request via Broadcast
  - handle_frame() — verarbeitet empfangene Frames (ARP + IPv4)
  - handle_arp() — lernt Sender-MAC, antwortet auf Requests an uns
  - resolve_mac() — ARP-Cache-Lookup
  - send_to() — sendet Frame an bekannte MAC
- 22/22 neue Netzwerk-Tests + 181/181 bestehende = 203/203 gesamt gruen

**Architektonische Bedeutung K10-K12:** Der Kernel hat jetzt alle Kernsubsysteme:
Boot, Memory, Interrupts, Capabilities, Prozesse, Scheduler, IPC, DID/Crypto,
Knowledge Graph, VFS, Syscalls, Timer/Clock, Block-Storage und Netzwerk.
Was noch fehlt: TCP/IP-Layer (auf K12 aufbauend), Userspace/Ring-3, und
echte Hardware-Treiber (HPET, virtio-blk, virtio-net) — aber die abstrakten
Schnittstellen sind alle definiert und getestet.

## K-Sprint 13: TCP/IP-Layer (03.08.2026)

`kernel/src/tcpip.rs` — IPv4, UDP, TCP, Routing, Sockets.

- `Ipv4Packet` — IPv4 mit Header-Checksumme, serialize/deserialize, with_checksum()
- `UdpPacket` — UDP, 8-Byte Header, serialize/deserialize
- `TcpSegment` — TCP mit Flags (SYN/ACK/FIN/RST/PSH/URG), serialize/deserialize
- `RoutingTable` — Longest Prefix Match, Default-Route, metric-basierte Sortierung
- `SocketManager` — UDP und TCP Sockets
  - UDP: bind/connect/send/recv/close, handle_udp() fuer eingehende Packets
  - TCP: bind/connect, State Machine (Listen→SynReceived→Established→CloseWait→Closed)
  - handle_tcp() — verarbeitet SYN/SYN-ACK/FIN, empfaengt Daten
- `IpStack` — verbindet NetworkStack + Routing + Sockets
  - handle_ipv4() — dispatcht UDP/TCP an SocketManager
  - handle_frame() — verarbeitet Ethernet→IPv4→UDP/TCP Stack
- 22/22 neue TCP/IP-Tests + 203/203 bestehende = 225/225 gesamt gruen

## K-Sprint 14: P2P-Consensus Foundation (03.08.2026)

`kernel/src/p2p.rs` — Peer-to-Peer Networking auf TCP/IP, Chain-ID 9000.

- `P2pMessage` — 9 Message-Types: Ping, Pong, Handshake, HandshakeAck,
  BlockAnnounce, TxAnnounce, Vote, PeerList, Bye. Serialisierung mit
  Chain-ID-Validierung (9000), DID-Feld, Timestamp.
- `PeerTable` — verwaltet Peers (IP, Port, DID, Status, Stats)
  - add/remove/find_by_addr, set_status/set_did/touch
  - Stat-Tracking: bytes_sent/recv, messages_sent/recv
  - max_peers-Limit, connected_count()
- `GossipProtocol` — Broadcast und Direct-Send
  - broadcast() — an alle verbundenen Peers
  - send_to() — an einen spezifischen Peer
  - handle_message() — verarbeitet Handshake/Bye, lernt DID
  - Peer-Discovery: make_peer_list() / handle_peer_list()
  - make_ping/pong/handshake() — Message-Factory
- `P2pNode` — Top-Level-Integration
  - connect_peer() — sendet Handshake
  - handle_handshake() — verarbeitet eingehenden Handshake, lernt DID, sendet Ack
  - ping_all() — Ping an alle Peers
  - announce_block() / announce_tx() — Block/Transaktion propagieren
  - disconnect_peer() — sauberer Disconnect mit Bye
- Chain-ID 9000 (Non-EVM, SHA-256) validiert in jeder Message
- 25/25 neue P2P-Tests + 225/225 bestehende = 250/250 gesamt gruen

**Architektonische Bedeutung:** Das ist die erste Blockchain-native Komponente
im Kernel. P2P-Networking mit DID-basiertem Handshake, Gossip-Protocol und
Chain-ID-Validierung — die Fundamente fuer Konsens und Block-Propagation.
Der Kernel ist jetzt nicht nur ein OS, sondern ein Blockchain-OS.

## K-Sprint 15: Security Layer (03.08.2026)

`kernel/src/security.rs` — Multi-Sig, Audit-Log, Reputation, Rate-Limiting, Secure-Channel.

1. **Multi-Signature Auth (ATC-18)** — `MultiSigManager` + `MultiSigProposal`
   - m-of-n Signatursammlung, Duplicate-Signer-Check, `is_ready()`, `execute()`
   - Signatur mit (DID, Ed25519-Signatur) — verknüpft mit K6/K6b

2. **Audit-Log (Tamper-Evident)** — Hash-Chain über alle Sicherheitseinträge
   - `log()` — seq + timestamp + actor + action + resource + result → hash
   - `verify_chain()` — revalidiert gesamte Hash-Kette
   - `filter_by_actor()` / `filter_by_result()`

3. **Peer-Reputation** — Score [-100..+100], automatischer Ban bei ≤ -50
   - `reward()` / `penalize()` / `unban()`, `is_banned()`, `banned_count()`

4. **Rate-Limiting (Token Bucket)** — pro-Peer Token-Bucket mit Refill
   - `allow(peer_id, now)` — konsumiert 1 Token, blockiert bei leerem Bucket
   - Zeitbasiertes Refill (tokens/sec)

5. **Secure-Channel** — verschlüsselte Kommunikation zwischen Peers
   - `establish()` / `send()` / `recv()` / `close()`
   - XOR-basiert für Tests, ersetztbar durch AEAD (XChaCha20-Poly1305)

6. **SecurityManager** — Top-Level Integration
   - `check_peer()` — Reputation + Rate-Limit kombiniert
   - `audit_log()` — zentrale Audit-Funktion
   - Verbindet Multi-Sig + Audit + Reputation + Rate-Limit + Channels

- 28/28 neue Security-Tests + 250/250 bestehende = 278/278 gesamt gruen

## K-Sprint 16: Konsens-Mechanismus (03.08.2026)

`kernel/src/consensus.rs` — DAG Consensus (ATC-04), Proof of History, Validator-Voting.

1. **Proof of History (PoH)** — `PohSequence`
   - Sequenzielle Hash-Kette für Zeitordnung (Solana-ähnlich)
   - `tick()` erzeugt Zeit-Tick, `record()` verknüpft Event-Hash
   - `verify()` revalidiert gesamte PoH-Kette ab Start-Hash

2. **DAG-Struktur (ATC-04)** — `Dag` + `DagVertex`
   - Vertices mit Mehrfach-Parents (parallele Transaktionen, kein Chain-Flaschenhals)
   - `add_vertex()` mit Parent-Existenz-Prüfung
   - `get_tips()` — unbestätigte Spitzen, `get_children()`, `topological_order()`
   - `tips_hash()` — Checkpoint-Hash über alle Tips
   - `confirm_vertex()` bei Finalität

3. **Validator-Registry** — `ValidatorRegistry` + `Validator`
   - Stake-basierte Registrierung, `select_proposer()` (weighted by stake)
   - `record_vote()` / `record_proposal()` — Stat-Tracking
   - `deactivate()` — Validator can be removed from consensus

4. **Vote-Pool & Finality** — `VotePool` + `Vote`
   - Stake-weighted 2/3 Supermajority für Finalität (configurable threshold)
   - `cast_vote()`, `is_final()`, `approve_count()` / `reject_count()`
   - `finalized_vertices()` — alle finalen Vertices

5. **Consensus-Engine** — `ConsensusEngine`
   - `init_genesis()` — DAG mit Genesis-Vertex initialisieren
   - `propose_vertex()` — neuen Vertex an Tips anhängen (PoH + Parents)
   - `vote()` / `handle_vote()` — abstimmen + automatische Bestätigung bei Finalität
   - `fork_choice()` — schwerester Pfad ab Genesis (meiste Votes)
   - `next_proposer()` — Stake-weighted Proposer-Selection via PoH-Hash

- 24/24 neue Konsens-Tests + 278/278 bestehende = 302/302 gesamt gruen

## K-Sprint 17: Memory-Pool & Transaction-Validation (03.08.2026)

`kernel/src/mempool.rs` — Mempool, Tx-Validator, Nonce-Tracking, State-DB.

- `Transaction` — 7 Tx-Types: Transfer, Stake, Unstake, Delegate, Vote, ContractCall, ContractDeploy
  - `gas_cost()` — Base-Gas + Payload-Gas (10 gas/byte)
  - `max_fee()` — gas_limit × gas_price
  - `to_bytes()` — Serialisierung für P2P-Propagation
- `MemoryPool` — verwaltet pendente Transaktionen
  - `add()` — mit Pool-Full und Duplicate-Check
  - `validate_tx()` — Gas-Limit, Recipient, Gas-Price Checks
  - `get_pending_batch(max)` — höchsten priorisierten Txs (gas_price × gas_limit)
  - `mark_in_dag()` / `mark_confirmed()` — Konsens-Integration
  - `cleanup(now)` — entfernt bestätigte/abgelaufene Txs
  - `txs_by_sender()` / `sender_nonce()` — per-Sender Tracking
- `NonceTracker` — verhindert Replay-Angriffe
  - `check_and_advance()` — Nonce muss sequenziell sein (0, 1, 2, ...)
  - `expected_nonce()` / `reset()`
- `StateDb` — vereinfachte Account-State-Datenbank
  - Balance, Staked, Nonce pro DID
  - `deposit()` / `withdraw()` / `stake()` / `unstake()`
  - `increment_nonce()` / `total_supply()`
- `TxValidator` — vollständige Transaktionsvalidierung
  - Gas-Price-Check, Gas-Limit-Check, Nonce-Check, Balance-Check
  - `validate()` — prüft alle Constraints vor Ausführung
  - `apply()` — wendet valide Tx auf State an (Transfer/Stake/Unstake)
- 30/30 neue Mempool-Tests + 302/302 bestehende = 332/332 gesamt gruen

## K-Sprint 18: Block-Proposal-Pipeline (03.08.2026)

`kernel/src/blockchain.rs` — Block, BlockChain, ProposalPipeline.

- `Block` — Block-Struktur mit height, parent_hash, transactions, tx_root (Merkle),
  state_root, gas_used, total_fees, Ed25519-Signatur
  - Deterministische Block-ID (Hash über alle Felder)
  - Merkle-Root über alle Tx-IDs
- `BlockChain` — lineare Block-Kette (Genesis → Block 1 → Block 2 → ...)
  - `add_genesis()` / `add_block()` mit Height- und Parent-Checks
  - `get_block(height)` / `get_by_hash()` / `last_block()`
- `ProposalPipeline` — verbindet Mempool → Block → DAG → Voting
  - `create_genesis()` — Genesis-Block + DAG-Vertex
  - `propose_block(max_txs)` — nimmt Txs aus Mempool, validiert, wendet auf State an,
    erzeugt Block, fügt zur Chain hinzu, propagiert als DAG-Vertex
  - `process_remote_block()` — verarbeitet eingehenden Block von anderem Node
  - `vote_on_block()` — stimmt über Block ab via Konsens
  - `cleanup_mempool()` — entfernt bestätigte Txs
- Volle Pipeline: Mempool (K17) → Block → Chain → DAG (K16) → Voting → Finality
- 20/20 neue Pipeline-Tests + 332/332 bestehende = 352/352 gesamt gruen

## K-Sprint 19: Contract VM / ShivaVM (03.08.2026)

`kernel/src/vm.rs` — Bytecode-Interpreter für Smart Contracts.

- **27 Opcodes**: Nop, Push, Pop, Add, Sub, Mul, Div, Mod, Eq, Ne, Lt, Gt, Lte, Gte,
  And, Or, Not, Jump, JumpIf, JumpIfNot, Call, Ret, Load, Store, Log, Self, Caller,
  Balance, Transfer, Halt
- **ShivaVM** — Stack-basierter Bytecode-Interpreter
  - 1024-Element Stack, Program Counter, Gas-Metering
  - `execute()` — interpretiert Bytecode Opcode für Opcode
  - `consume_gas()` — pro-Opcode Gas-Kosten, OutOfGas-Abort
  - `ExecResult` — success, return_value, gas_used, gas_refund, logs, storage_changes
- **ContractStorage** — Key-Value Store pro Contract (contract_addr, key) → value
  - `load()` / `store()` — persistent über Calls hinweg
  - `clear_contract()` — für Self-Destruct
- **ContractRegistry** — verwaltet deployte Contracts
  - `deploy()` / `get()` / `exists()` / `count()`
  - `deposit()` / `withdraw()` / `balance()` — Contract-Balances
- **VmEngine** — Top-Level Integration
  - `deploy()` — Contract deployen (Bytecode + Owner-DID)
  - `call()` — Contract aufrufen (erzeugt ShivaVM, executes, returns ExecResult)
- 30/30 neue VM-Tests + 352/352 bestehende = 382/382 gesamt gruen

## K-Sprint 20: Contract-Call-Integration (03.08.2026)

`kernel/src/contract.rs` — Verbindet ShivaVM (K19) mit Block-Pipeline (K18).

- `ContractExecutor` — verarbeitet Contract-Transaktionen
  - `process_deploy()` — ContractDeploy-Tx → Bytecode extrahieren → VmEngine.deploy()
    - Contract-Adresse = hash(sender_did + nonce + poh_hash) → `did:contract:<hex>`
    - Deterministisch: gleiche Sender+Nonce → gleiche Adresse
  - `process_call()` — ContractCall-Tx → Contract-Adresse extrahieren → VmEngine.call()
    - Gas-Limit aus Transaction, Caller-DID weitergereicht
    - ExecResult (return_value, gas_used, logs, storage_changes)
  - `process_tx()` — Dispatch je nach TxType (Deploy/Call/Other)
- Payload-Format:
  - Deploy: `[bytecode_len(4)] [bytecode]`
  - Call: `[contract_addr_len(2)] [contract_addr] [call_data...]`
- `build_deploy_payload()` / `build_call_payload()` — Hilfsfunktionen
- Full Workflow: Deploy (Init) → Call (Increment) → State persists across calls
- 17/17 neue Contract-Tests + 382/382 bestehende = 399/399 gesamt gruen
