//! ShivaCore Kernel — MemoryManager Trait Implementation.
//!
//! Verbindet den Boot-Level Heap-Allocator (allocator.rs) mit dem
//! ats1000.rs MemoryManager-Trait. Bietet prozessbezogene
//! Speicherverwaltung mit Capability-Integration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::ats1000::{MemoryManager, MemRegion, Pid};
use crate::capability::{CapabilityTable, Pid as CapPid, ResourceType, Rights};

/// Verwaltete Speicherregion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocatedRegion {
    pub addr: u64,
    pub size: u64,
    pub owner_pid: Pid,
    pub region_id: u64,
}

/// Fehler bei Speicheroperationen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemError {
    OutOfMemory,
    InvalidRegion,
    NoCapability,
    DoubleFree,
    InvalidAlignment,
}

/// Kernel Memory Manager — verwaltet Speicherregionen pro Prozess.
/// Nutzt Capability-System fuer Zugriffskontrolle.
pub struct KernelMemoryManager {
    /// Naechste virtuelle Adresse (simple Bump-Allocation fuer Userspace-Simulation)
    next_addr: AtomicU64,
    /// Alle aktiven Regionen: region_id -> AllocatedRegion
    regions: BTreeMap<u64, AllocatedRegion>,
    /// Zaehler fuer Region-IDs
    next_region_id: AtomicU64,
    /// Gesamter allozierter Speicher (Bytes)
    total_allocated: u64,
    /// Peak-Speichernutzung
    peak_allocated: u64,
}

const HEAP_BASE: u64 = 0x_4444_4444_0000;
const HEAP_MAX: u64 = 100 * 1024 * 1024; // 100 MiB Simulations-Limit

impl KernelMemoryManager {
    pub fn new() -> Self {
        Self {
            next_addr: AtomicU64::new(HEAP_BASE),
            regions: BTreeMap::new(),
            next_region_id: AtomicU64::new(1),
            total_allocated: 0,
            peak_allocated: 0,
        }
    }

    /// Allokiert eine Speicherregion fuer einen Prozess.
    /// Vergibt automatisch READ + WRITE + EXEC Capability.
    pub fn allocate(
        &mut self,
        caps: &mut CapabilityTable,
        pid: Pid,
        size: u64,
    ) -> Result<AllocatedRegion, MemError> {
        if size == 0 { return Err(MemError::InvalidAlignment); }

        let current = self.next_addr.load(Ordering::SeqCst);
        if current + size > HEAP_BASE + HEAP_MAX {
            return Err(MemError::OutOfMemory);
        }

        let region_id = self.next_region_id.fetch_add(1, Ordering::SeqCst);
        let addr = self.next_addr.fetch_add(size, Ordering::SeqCst);

        // 4KB Alignment
        let aligned_addr = (addr + 0xFFF) & !0xFFF;
        if aligned_addr != addr {
            self.next_addr.store(aligned_addr + size, Ordering::SeqCst);
        }

        let region = AllocatedRegion {
            addr: aligned_addr,
            size,
            owner_pid: pid,
            region_id,
        };

        // Capability vergeben
        caps.create(
            CapPid(pid), ResourceType::Memory, region_id,
            Rights::READ | Rights::WRITE | Rights::EXEC | Rights::DELEGATE,
        );

        self.total_allocated += size;
        if self.total_allocated > self.peak_allocated {
            self.peak_allocated = self.total_allocated;
        }

        self.regions.insert(region_id, region);
        Ok(region)
    }

    /// Gibt eine Speicherregion frei.
    /// Prueft WRITE-Capability und widerruft alle Capabilities.
    pub fn deallocate(
        &mut self,
        caps: &mut CapabilityTable,
        pid: Pid,
        region_id: u64,
    ) -> Result<(), MemError> {
        // Existiert die Region?
        let region = self.regions.get(&region_id).ok_or(MemError::InvalidRegion)?;

        // Capability-Check
        if !caps.check(CapPid(pid), ResourceType::Memory, region_id, Rights::WRITE) {
            return Err(MemError::NoCapability);
        }

        let size = region.size;
        self.total_allocated -= size;
        self.regions.remove(&region_id);

        // Capabilities widerrufen
        let cap_ids: Vec<_> = caps.list_for(CapPid(pid)).iter()
            .filter(|c| c.resource_type == ResourceType::Memory && c.resource_id == region_id)
            .map(|c| c.id)
            .collect();
        for cap_id in cap_ids {
            caps.revoke(cap_id);
        }

        Ok(())
    }

    /// Liest Speicher (simuliert — gibt die Regions-ID zurueck, kein echter Memory-Mapping).
    /// Prueft READ-Capability.
    pub fn read_check(
        &self,
        caps: &CapabilityTable,
        pid: Pid,
        region_id: u64,
    ) -> Result<AllocatedRegion, MemError> {
        let region = self.regions.get(&region_id).ok_or(MemError::InvalidRegion)?;
        if !caps.check(CapPid(pid), ResourceType::Memory, region_id, Rights::READ) {
            return Err(MemError::NoCapability);
        }
        Ok(*region)
    }

    /// Mappt eine physikalische Adresse (mmap — simuliert).
    pub fn mmap(
        &mut self,
        caps: &mut CapabilityTable,
        pid: Pid,
        size: u64,
    ) -> Result<AllocatedRegion, MemError> {
        self.allocate(caps, pid, size)
    }

    /// Statistik
    pub fn stats(&self) -> MemStats {
        MemStats {
            total_allocated: self.total_allocated,
            peak_allocated: self.peak_allocated,
            active_regions: self.regions.len() as u64,
            heap_base: HEAP_BASE,
            heap_max: HEAP_MAX,
        }
    }

    /// Liste aller Regionen eines Prozesses
    pub fn regions_for(&self, pid: Pid) -> Vec<AllocatedRegion> {
        self.regions.values()
            .filter(|r| r.owner_pid == pid)
            .cloned()
            .collect()
    }
}

/// Memory-Statistik
#[derive(Debug, Clone, Copy)]
pub struct MemStats {
    pub total_allocated: u64,
    pub peak_allocated: u64,
    pub active_regions: u64,
    pub heap_base: u64,
    pub heap_max: u64,
}

/// ats1000 MemoryManager-Trait-Implementierung (Adapter)
impl MemoryManager for KernelMemoryManager {
    fn alloc(&mut self, size: u64, pid: Pid) -> Option<MemRegion> {
        // Vereinfachte Version ohne Capability-Check (direkt im Trait)
        if size == 0 { return None; }
        let current = self.next_addr.load(Ordering::SeqCst);
        if current + size > HEAP_BASE + HEAP_MAX { return None; }

        let region_id = self.next_region_id.fetch_add(1, Ordering::SeqCst);
        let addr = self.next_addr.fetch_add(size, Ordering::SeqCst);
        let aligned_addr = (addr + 0xFFF) & !0xFFF;
        if aligned_addr != addr {
            self.next_addr.store(aligned_addr + size, Ordering::SeqCst);
        }

        let region = AllocatedRegion {
            addr: aligned_addr, size, owner_pid: pid, region_id,
        };
        self.total_allocated += size;
        if self.total_allocated > self.peak_allocated {
            self.peak_allocated = self.total_allocated;
        }
        self.regions.insert(region_id, region);
        Some(MemRegion { addr: aligned_addr, size, pid })
    }

    fn free(&mut self, region: MemRegion) -> bool {
        // Finde Region mit passender Adresse und PID
        let region_id = self.regions.iter()
            .find(|(_, r)| r.addr == region.addr && r.owner_pid == region.pid)
            .map(|(id, _)| *id);

        if let Some(id) = region_id {
            let size = self.regions.get(&id).unwrap().size;
            self.total_allocated -= size;
            self.regions.remove(&id);
            true
        } else {
            false
        }
    }

    fn mmap(&mut self, addr: u64, size: u64) -> Option<MemRegion> {
        // Ignoriert addr (Bump-Allocator), allokiert size
        // addr wird als Hint behandelt
        let _ = addr;
        self.alloc(size, 0) // PID 0 = Kernel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::CapabilityTable;

    fn pid(n: u32) -> Pid { n }

    #[test]
    fn test_allocate_region() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let r = mm.allocate(&mut caps, pid(1), 4096).unwrap();
        assert_eq!(r.size, 4096);
        assert_eq!(r.owner_pid, pid(1));
        assert!(r.addr >= HEAP_BASE);
    }

    #[test]
    fn test_allocate_creates_capability() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let r = mm.allocate(&mut caps, pid(1), 4096).unwrap();
        assert!(caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, r.region_id, Rights::READ));
        assert!(caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, r.region_id, Rights::WRITE));
        assert!(caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, r.region_id, Rights::EXEC));
    }

    #[test]
    fn test_deallocate_revokes_caps() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let r = mm.allocate(&mut caps, pid(1), 8192).unwrap();
        let rid = r.region_id;

        // Cap existiert
        assert!(caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, rid, Rights::WRITE));

        mm.deallocate(&mut caps, pid(1), rid).unwrap();

        // Cap widerrufen
        assert!(!caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, rid, Rights::WRITE));
        assert!(!caps.check(crate::capability::Pid(pid(1)), ResourceType::Memory, rid, Rights::READ));
    }

    #[test]
    fn test_deallocate_without_cap_rejected() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let r = mm.allocate(&mut caps, pid(1), 4096).unwrap();

        // pid(2) hat keine Cap
        let result = mm.deallocate(&mut caps, pid(2), r.region_id);
        assert_eq!(result, Err(MemError::NoCapability));
    }

    #[test]
    fn test_deallocate_invalid_region() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let result = mm.deallocate(&mut caps, pid(1), 999);
        assert_eq!(result, Err(MemError::InvalidRegion));
    }

    #[test]
    fn test_multiple_allocations_unique_addresses() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();

        let r1 = mm.allocate(&mut caps, pid(1), 4096).unwrap();
        let r2 = mm.allocate(&mut caps, pid(1), 4096).unwrap();
        let r3 = mm.allocate(&mut caps, pid(2), 4096).unwrap();

        assert_ne!(r1.addr, r2.addr);
        assert_ne!(r2.addr, r3.addr);
        assert_ne!(r1.addr, r3.addr);
    }

    #[test]
    fn test_read_check_without_cap_rejected() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        let r = mm.allocate(&mut caps, pid(1), 4096).unwrap();

        // pid(1) kann lesen
        assert!(mm.read_check(&caps, pid(1), r.region_id).is_ok());
        // pid(2) kann nicht lesen
        assert_eq!(mm.read_check(&caps, pid(2), r.region_id), Err(MemError::NoCapability));
    }

    #[test]
    fn test_stats_tracking() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();

        mm.allocate(&mut caps, pid(1), 4096).unwrap();
        mm.allocate(&mut caps, pid(1), 8192).unwrap();
        mm.allocate(&mut caps, pid(2), 4096).unwrap();

        let s = mm.stats();
        assert_eq!(s.total_allocated, 16384);
        assert_eq!(s.active_regions, 3);
        assert_eq!(s.peak_allocated, 16384);
    }

    #[test]
    fn test_stats_peak_after_free() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();

        let r = mm.allocate(&mut caps, pid(1), 8192).unwrap();
        let s1 = mm.stats();
        assert_eq!(s1.peak_allocated, 8192);

        mm.deallocate(&mut caps, pid(1), r.region_id).unwrap();
        let s2 = mm.stats();
        assert_eq!(s2.total_allocated, 0);
        assert_eq!(s2.peak_allocated, 8192); // Peak bleibt
    }

    #[test]
    fn test_regions_for_pid() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();

        mm.allocate(&mut caps, pid(1), 4096).unwrap();
        mm.allocate(&mut caps, pid(1), 4096).unwrap();
        mm.allocate(&mut caps, pid(2), 4096).unwrap();

        let r1 = mm.regions_for(pid(1));
        let r2 = mm.regions_for(pid(2));
        assert_eq!(r1.len(), 2);
        assert_eq!(r2.len(), 1);
    }

    #[test]
    fn test_ats1000_trait_impl() {
        let mut mm = KernelMemoryManager::new();
        let r = mm.alloc(4096, pid(1));
        assert!(r.is_some());
        let region = r.unwrap();
        assert_eq!(region.size, 4096);
        assert_eq!(region.pid, pid(1));

        // Free via trait
        assert!(mm.free(region));
        // Double-free fails
        assert!(!mm.free(region));
    }

    #[test]
    fn test_ats1000_mmap() {
        let mut mm = KernelMemoryManager::new();
        let r = MemoryManager::mmap(&mut mm, 0x5000, 4096).unwrap();
        assert_eq!(r.size, 4096);
        assert!(r.addr >= HEAP_BASE);
    }

    #[test]
    fn test_zero_size_rejected() {
        let mut mm = KernelMemoryManager::new();
        let mut caps = CapabilityTable::new();
        assert_eq!(mm.allocate(&mut caps, pid(1), 0), Err(MemError::InvalidAlignment));
    }
}
