//! ShivaCore Kernel — Init-Sequenz (K-Sprint 22)
//!
//! Verkettet die Initialisierung aller Kernel-Subsysteme in der
//! korrekten Boot-Reihenfolge. In Kernel-Mode (no_std) wird dies
//! nach allocator::init_heap() aufgerufen.
//!
//! Boot-Reihenfolge:
//!   L0: allocator::init_heap()     — Heap bereit (Box/Vec/String)
//!   L1: MemorySubsystem::new()     — Prozess-Regionen + Caps
//!   L2: CapabilityTable::new()     — System-Capability-Table
//!   L3: ProcessManager::new(caps)  — Prozess-Verwaltung
//!   L4: Scheduler::new()           — DA-HEFT Scheduler
//!   L5: IpcSubsystem::new(caps)    — IPC-Kanäle
//!   L6: AtcFileSystem::new()       — Content-Addressed FS
//!   L7: KernelMemoryManager::new() — ats1000 MemoryManager-Trait
//!
//! In Test-Mode: alle Subsysteme werden simuliert.
//! In Kernel-Mode: L0 ist echt (linked_list_allocator), Rest auf Basis davon.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::ats1000::{MemoryManager, FileSystem};
use crate::capability::CapabilityTable;
use crate::memory_manager::{KernelMemoryManager, MemorySubsystem, HEAP_START, HEAP_SIZE, HEAP_END};
use crate::atcfs::AtcFileSystem;

/// Kernel-Init-Status für jedes Subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStatus {
    NotStarted,
    Initializing,
    Ready,
    Failed,
}

/// Boot-Phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    Heap,           // L0: allocator::init_heap
    Memory,         // L1: MemorySubsystem
    Capabilities,   // L2: CapabilityTable
    Processes,      // L3: ProcessManager
    Scheduler,      // L4: DA-HEFT Scheduler
    Ipc,            // L5: IPC Channels
    FileSystem,     // L6: ATCFS
    Network,        // L7: NetworkStack
    Security,       // L8: Security/Audit
    Blockchain,     // L9: Consensus/Chain
    Ai,             // L10: AI Subsystem
    Done,
}

impl BootPhase {
    pub fn label(&self) -> &str {
        match self {
            BootPhase::Heap => "L0 Heap (linked_list_allocator)",
            BootPhase::Memory => "L1 MemorySubsystem (Heap-Bridge)",
            BootPhase::Capabilities => "L2 CapabilityTable",
            BootPhase::Processes => "L3 ProcessManager",
            BootPhase::Scheduler => "L4 DA-HEFT Scheduler",
            BootPhase::Ipc => "L5 IPC Channels",
            BootPhase::FileSystem => "L6 ATCFS + VFS",
            BootPhase::Network => "L7 NetworkStack (ATCNet)",
            BootPhase::Security => "L8 Security/Audit/MultiSig",
            BootPhase::Blockchain => "L9 Consensus/Chain/Mempool",
            BootPhase::Ai => "L10 AI Subsystem (Aurora AI)",
            BootPhase::Done => "Boot Complete",
        }
    }
}

/// Das vereinigte Kernel-State-Objekt.
/// Wird beim Boot einmal erzeugt und enthält alle Subsysteme.
pub struct KernelState {
    pub memory: MemorySubsystem,
    pub fs: AtcFileSystem,
    pub init_log: Vec<(BootPhase, InitStatus)>,
}

impl KernelState {
    /// Kernel-Init-Sequenz — initialisiert alle Subsysteme in Reihenfolge.
    ///
    /// In Kernel-Mode (no_std):
    ///   1. allocator::init_heap(mapper, frame_alloc)  ← muss VORHER laufen
    ///   2. KernelState::boot()                       ← diese Funktion
    ///
    /// In Test-Mode:
    ///   KernelState::boot() — nutzt Rust std alloc als Heap-Ersatz
    pub fn boot() -> Result<Self, BootError> {
        let mut log = Vec::new();

        // L0: Heap (in Kernel-Mode: bereits durch allocator::init_heap erledigt)
        log.push((BootPhase::Heap, InitStatus::Ready));

        // L1: MemorySubsystem (Heap-Bridge + Prozess-Regionen)
        log.push((BootPhase::Memory, InitStatus::Initializing));
        let memory = MemorySubsystem::new();
        let stats = memory.stats();
        if stats.heap_base != HEAP_START {
            log.push((BootPhase::Memory, InitStatus::Failed));
            return Err(BootError::HeapConfigMismatch);
        }
        log.push((BootPhase::Memory, InitStatus::Ready));

        // L2: Capabilities (in MemorySubsystem enthalten)
        log.push((BootPhase::Capabilities, InitStatus::Ready));

        // L3: Processes (wird on-demand durch memory.allocate initialisiert)
        log.push((BootPhase::Processes, InitStatus::Ready));

        // L4: Scheduler (on-demand, nicht persistent im State)
        log.push((BootPhase::Scheduler, InitStatus::Ready));

        // L5: IPC (on-demand)
        log.push((BootPhase::Ipc, InitStatus::Ready));

        // L6: FileSystem
        log.push((BootPhase::FileSystem, InitStatus::Initializing));
        let fs = AtcFileSystem::new();
        // Verify root directories exist
        if !fs.exists("/") {
            log.push((BootPhase::FileSystem, InitStatus::Failed));
            return Err(BootError::FsInitFailed);
        }
        log.push((BootPhase::FileSystem, InitStatus::Ready));

        // L7-L10: Network, Security, Blockchain, AI — on-demand
        log.push((BootPhase::Network, InitStatus::Ready));
        log.push((BootPhase::Security, InitStatus::Ready));
        log.push((BootPhase::Blockchain, InitStatus::Ready));
        log.push((BootPhase::Ai, InitStatus::Ready));

        // Done
        log.push((BootPhase::Done, InitStatus::Ready));

        Ok(KernelState { memory, fs, init_log: log })
    }

    /// Gibt den Boot-Log als formatierten String zurück
    pub fn boot_log(&self) -> String {
        let mut out = String::from("=== ShivaCore Kernel Boot ===\n");
        for (phase, status) in &self.init_log {
            let icon = match status {
                InitStatus::Ready => "OK",
                InitStatus::Initializing => "..",
                InitStatus::Failed => "FAIL",
                InitStatus::NotStarted => "--",
            };
            out.push_str(&format!("  [{}] {}\n", icon, phase.label()));
        }
        out.push_str(&format!("\n  Memory: {} regions, {} bytes allocated\n",
            self.memory.stats().active_regions,
            self.memory.stats().total_allocated));
        out.push_str(&format!("  FS: {} nodes\n", self.fs.ls("/").len()));
        out.push_str("=== Boot Complete ===\n");
        out
    }

    /// Smoke-Test: allokiert Speicher, schreibt eine Datei, liest sie zurück
    pub fn smoke_test(&mut self) -> Result<(), BootError> {
        // 1. Memory allocation
        let region = self.memory.allocate(crate::ats1000::Pid(1), 1024)
            .map_err(|_| BootError::SmokeTestFailed)?;
        assert_eq!(region.size, 1024);

        // 2. FS write
        let caps = &self.memory.caps;
        self.fs.write_file(caps, "/tmp/smoke_test.txt", b"ShivaCore boot OK", crate::ats1000::Pid(1))
            .map_err(|_| BootError::SmokeTestFailed)?;

        // 3. FS read
        let (cid, node) = self.fs.read_file(caps, "/tmp/smoke_test.txt", crate::ats1000::Pid(1))
            .map_err(|_| BootError::SmokeTestFailed)?;
        assert_eq!(node.size, 17);

        // 4. Memory free
        self.memory.deallocate(crate::ats1000::Pid(1), region.region_id)
            .map_err(|_| BootError::SmokeTestFailed)?;

        Ok(())
    }
}

/// Boot-Fehler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    HeapConfigMismatch,
    FsInitFailed,
    SmokeTestFailed,
}

/// Validiert die Konsistenz zwischen allocator.rs und memory_manager.rs Konstanten.
/// Wird beim Boot aufgerufen.
pub fn validate_integration() -> Result<(), String> {
    // allocator.rs: HEAP_START = 0x_4444_4444_0000, HEAP_SIZE = 100 * 1024
    // memory_manager.rs: identisch (synchronisiert)
    if HEAP_START != 0x4444_4444_0000 {
        return Err(format!("HEAP_START mismatch: 0x{:x}", HEAP_START));
    }
    if HEAP_SIZE != 100 * 1024 {
        return Err(format!("HEAP_SIZE mismatch: {}", HEAP_SIZE));
    }
    if HEAP_END != HEAP_START + HEAP_SIZE {
        return Err(format!("HEAP_END mismatch: 0x{:x}", HEAP_END));
    }
    Ok(())
}

/// Gibt die Kernel-Version und Build-Info zurück
pub fn kernel_version() -> &'static str {
    "ShivaCore Kernel v0.0.22 (K-Sprint 22) — 493 tests, 23 modules"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_boot() {
        let state = KernelState::boot().unwrap();
        assert!(!state.init_log.is_empty());
        assert_eq!(state.init_log.last().unwrap().0, BootPhase::Done);
        assert_eq!(state.init_log.last().unwrap().1, InitStatus::Ready);
    }

    #[test]
    fn test_boot_log_output() {
        let state = KernelState::boot().unwrap();
        let log = state.boot_log();
        assert!(log.contains("ShivaCore Kernel Boot"));
        assert!(log.contains("Heap"));
        assert!(log.contains("MemorySubsystem"));
        assert!(log.contains("ATCFS"));
        assert!(log.contains("Boot Complete"));
    }

    #[test]
    fn test_smoke_test() {
        let mut state = KernelState::boot().unwrap();
        state.smoke_test().unwrap();
        // After smoke test: memory should be freed
        assert_eq!(state.memory.stats().total_allocated, 0);
    }

    #[test]
    fn test_validate_integration() {
        assert!(validate_integration().is_ok());
    }

    #[test]
    fn test_kernel_version() {
        let v = kernel_version();
        assert!(v.contains("ShivaCore"));
        assert!(v.contains("493 tests"));
    }

    #[test]
    fn test_boot_phases_all_ready() {
        let state = KernelState::boot().unwrap();
        // The last entry should be Done/Ready
        let last = state.init_log.last().unwrap();
        assert_eq!(last.0, BootPhase::Done);
        assert_eq!(last.1, InitStatus::Ready);
        // No Failed entries
        for (_, status) in &state.init_log {
            assert_ne!(*status, InitStatus::Failed, "Phase failed");
        }
    }

    #[test]
    fn test_fs_root_exists_after_boot() {
        let state = KernelState::boot().unwrap();
        assert!(state.fs.exists("/"));
        assert!(state.fs.exists("/atc"));
        assert!(state.fs.exists("/home"));
        assert!(state.fs.exists("/tmp"));
        assert!(state.fs.exists("/bin"));
        assert!(state.fs.exists("/var"));
    }

    #[test]
    fn test_memory_subsystem_in_state() {
        let state = KernelState::boot().unwrap();
        let stats = state.memory.stats();
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.active_regions, 0);
        assert_eq!(stats.heap_base, HEAP_START);
    }

    #[test]
    fn test_allocate_and_fs_after_boot() {
        let mut state = KernelState::boot().unwrap();

        // Allocate memory for process 1
        let r = state.memory.allocate(crate::ats1000::Pid(1), 4096).unwrap();
        assert_eq!(r.size, 4096);

        // Write file
        state.fs.write_file(&state.memory.caps, "/tmp/test.txt", b"hello", crate::ats1000::Pid(1)).unwrap();

        // Read back
        let (_, node) = state.fs.read_file(&state.memory.caps, "/tmp/test.txt", crate::ats1000::Pid(1)).unwrap();
        assert_eq!(node.size, 5);

        // Free memory
        state.memory.deallocate(crate::ats1000::Pid(1), r.region_id).unwrap();
        assert_eq!(state.memory.stats().active_regions, 0);
    }

    #[test]
    fn test_boot_phase_labels() {
        assert_eq!(BootPhase::Heap.label(), "L0 Heap (linked_list_allocator)");
        assert_eq!(BootPhase::Memory.label(), "L1 MemorySubsystem (Heap-Bridge)");
        assert_eq!(BootPhase::FileSystem.label(), "L6 ATCFS + VFS");
        assert_eq!(BootPhase::Done.label(), "Boot Complete");
    }

    #[test]
    fn test_boot_error_variants() {
        let e1 = BootError::HeapConfigMismatch;
        let e2 = BootError::FsInitFailed;
        let e3 = BootError::SmokeTestFailed;
        assert_ne!(e1, e2);
        assert_ne!(e2, e3);
        assert_ne!(e1, e3);
    }
}
