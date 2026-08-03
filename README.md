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
