use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(feature = "snapshot-experiment-alloc")]
use std::ptr;
#[cfg(feature = "snapshot-experiment-alloc")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "snapshot-experiment-alloc")]
static COUNTERS: AllocationCounters = AllocationCounters::new();

/// Process-global allocator used only by snapshot benchmark experiments.
///
/// It delegates storage management to [`System`] and records saturating
/// process-wide counters. The counters are observational: they never affect
/// allocation success or ownership.
pub struct HbkSnapshotExperimentAllocator;

// SAFETY: Every non-zero, non-null allocation operation is delegated to
// `System` with the same valid layout/pointer contract. The additional state is
// composed only of atomics and never dereferences or owns allocated pointers.
unsafe impl GlobalAlloc for HbkSnapshotExperimentAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(not(feature = "snapshot-experiment-alloc"))]
        {
            // SAFETY: The wrapper preserves the caller-provided `GlobalAlloc`
            // contract and delegates the unchanged layout directly to
            // `System`.
            return unsafe { System.alloc(layout) };
        }
        #[cfg(feature = "snapshot-experiment-alloc")]
        {
            COUNTERS.record_allocation_call();
            if layout.size() == 0 {
                return ptr::null_mut();
            }
            // SAFETY: `GlobalAlloc::alloc` requires the caller to provide a valid,
            // non-zero layout; that unchanged layout is delegated to `System`.
            let allocated = unsafe { System.alloc(layout) };
            if !allocated.is_null() {
                COUNTERS.record_allocation(layout.size() as u64);
            }
            allocated
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        #[cfg(not(feature = "snapshot-experiment-alloc"))]
        {
            // SAFETY: The wrapper preserves the caller-provided `GlobalAlloc`
            // contract and delegates the unchanged layout directly to
            // `System`.
            return unsafe { System.alloc_zeroed(layout) };
        }
        #[cfg(feature = "snapshot-experiment-alloc")]
        {
            COUNTERS.record_allocation_call();
            if layout.size() == 0 {
                return ptr::null_mut();
            }
            // SAFETY: `GlobalAlloc::alloc_zeroed` requires the caller to provide a
            // valid, non-zero layout; that unchanged layout is delegated to
            // `System`.
            let allocated = unsafe { System.alloc_zeroed(layout) };
            if !allocated.is_null() {
                COUNTERS.record_allocation(layout.size() as u64);
            }
            allocated
        }
    }

    unsafe fn dealloc(&self, allocated: *mut u8, layout: Layout) {
        #[cfg(not(feature = "snapshot-experiment-alloc"))]
        {
            // SAFETY: The wrapper preserves the caller-provided `GlobalAlloc`
            // contract and delegates the unchanged pointer/layout directly to
            // `System`.
            unsafe { System.dealloc(allocated, layout) };
        }
        #[cfg(feature = "snapshot-experiment-alloc")]
        {
            COUNTERS.record_deallocation_call();
            if allocated.is_null() || layout.size() == 0 {
                return;
            }
            // SAFETY: `GlobalAlloc::dealloc` requires `allocated` to denote a live
            // block obtained from this allocator with `layout`. This allocator
            // delegates all such blocks to `System`, so the same pair is valid
            // there.
            unsafe { System.dealloc(allocated, layout) };
            COUNTERS.record_deallocation(layout.size() as u64);
        }
    }

    unsafe fn realloc(&self, allocated: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(not(feature = "snapshot-experiment-alloc"))]
        {
            // SAFETY: The wrapper preserves the caller-provided `GlobalAlloc`
            // contract and delegates the unchanged pointer, layout and size
            // directly to `System`.
            return unsafe { System.realloc(allocated, layout, new_size) };
        }
        #[cfg(feature = "snapshot-experiment-alloc")]
        {
            COUNTERS.record_reallocation_call();
            if new_size == 0 {
                if !allocated.is_null() && layout.size() != 0 {
                    // SAFETY: A non-null block passed to `realloc` must be a live
                    // allocation from this allocator. Delegation means it is also
                    // a live `System` allocation with the same layout.
                    unsafe { System.dealloc(allocated, layout) };
                    COUNTERS.record_deallocation(layout.size() as u64);
                }
                return ptr::null_mut();
            }
            if allocated.is_null() {
                let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
                    return ptr::null_mut();
                };
                // SAFETY: `new_layout` was validated above and has non-zero size.
                let replacement = unsafe { System.alloc(new_layout) };
                if !replacement.is_null() {
                    COUNTERS.record_allocation(new_size as u64);
                }
                return replacement;
            }
            if layout.size() == 0 {
                return ptr::null_mut();
            }

            // SAFETY: A non-null block passed to `realloc` must be a live
            // allocation obtained from this allocator using `layout`; this
            // allocator delegates it to `System`. `new_size` is non-zero.
            let replacement = unsafe { System.realloc(allocated, layout, new_size) };
            if !replacement.is_null() {
                COUNTERS.record_reallocation(layout.size() as u64, new_size as u64);
            }
            replacement
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HbkSnapshotExperimentAllocationSnapshot {
    pub allocation_calls: u64,
    pub reallocation_calls: u64,
    pub deallocation_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
    pub current_live_bytes: u64,
    pub peak_live_bytes: u64,
}

impl HbkSnapshotExperimentAllocationSnapshot {
    pub fn delta_since(self, earlier: Self) -> HbkSnapshotExperimentAllocationDelta {
        HbkSnapshotExperimentAllocationDelta {
            allocation_calls: self
                .allocation_calls
                .saturating_sub(earlier.allocation_calls),
            reallocation_calls: self
                .reallocation_calls
                .saturating_sub(earlier.reallocation_calls),
            deallocation_calls: self
                .deallocation_calls
                .saturating_sub(earlier.deallocation_calls),
            allocated_bytes: self.allocated_bytes.saturating_sub(earlier.allocated_bytes),
            deallocated_bytes: self
                .deallocated_bytes
                .saturating_sub(earlier.deallocated_bytes),
            live_bytes_before: earlier.current_live_bytes,
            live_bytes_after: self.current_live_bytes,
            peak_live_bytes_before: earlier.peak_live_bytes,
            peak_live_bytes_after: self.peak_live_bytes,
            peak_live_bytes_growth: self.peak_live_bytes.saturating_sub(earlier.peak_live_bytes),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HbkSnapshotExperimentAllocationDelta {
    pub allocation_calls: u64,
    pub reallocation_calls: u64,
    pub deallocation_calls: u64,
    pub allocated_bytes: u64,
    pub deallocated_bytes: u64,
    pub live_bytes_before: u64,
    pub live_bytes_after: u64,
    pub peak_live_bytes_before: u64,
    pub peak_live_bytes_after: u64,
    pub peak_live_bytes_growth: u64,
}

pub fn experiment_allocation_snapshot() -> HbkSnapshotExperimentAllocationSnapshot {
    #[cfg(not(feature = "snapshot-experiment-alloc"))]
    {
        HbkSnapshotExperimentAllocationSnapshot::default()
    }
    #[cfg(feature = "snapshot-experiment-alloc")]
    {
        COUNTERS.snapshot()
    }
}

#[cfg(feature = "snapshot-experiment-alloc")]
struct AllocationCounters {
    allocation_calls: AtomicU64,
    reallocation_calls: AtomicU64,
    deallocation_calls: AtomicU64,
    allocated_bytes: AtomicU64,
    deallocated_bytes: AtomicU64,
    current_live_bytes: AtomicU64,
    peak_live_bytes: AtomicU64,
}

#[cfg(feature = "snapshot-experiment-alloc")]
impl AllocationCounters {
    const fn new() -> Self {
        Self {
            allocation_calls: AtomicU64::new(0),
            reallocation_calls: AtomicU64::new(0),
            deallocation_calls: AtomicU64::new(0),
            allocated_bytes: AtomicU64::new(0),
            deallocated_bytes: AtomicU64::new(0),
            current_live_bytes: AtomicU64::new(0),
            peak_live_bytes: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> HbkSnapshotExperimentAllocationSnapshot {
        HbkSnapshotExperimentAllocationSnapshot {
            allocation_calls: self.allocation_calls.load(Ordering::Relaxed),
            reallocation_calls: self.reallocation_calls.load(Ordering::Relaxed),
            deallocation_calls: self.deallocation_calls.load(Ordering::Relaxed),
            allocated_bytes: self.allocated_bytes.load(Ordering::Relaxed),
            deallocated_bytes: self.deallocated_bytes.load(Ordering::Relaxed),
            current_live_bytes: self.current_live_bytes.load(Ordering::Relaxed),
            peak_live_bytes: self.peak_live_bytes.load(Ordering::Relaxed),
        }
    }

    fn record_allocation_call(&self) {
        saturating_add(&self.allocation_calls, 1);
    }

    fn record_reallocation_call(&self) {
        saturating_add(&self.reallocation_calls, 1);
    }

    fn record_deallocation_call(&self) {
        saturating_add(&self.deallocation_calls, 1);
    }

    fn record_allocation(&self, bytes: u64) {
        saturating_add(&self.allocated_bytes, bytes);
        let live = saturating_add(&self.current_live_bytes, bytes).saturating_add(bytes);
        update_peak(&self.peak_live_bytes, live);
    }

    fn record_deallocation(&self, bytes: u64) {
        saturating_add(&self.deallocated_bytes, bytes);
        saturating_sub(&self.current_live_bytes, bytes);
    }

    fn record_reallocation(&self, old_bytes: u64, new_bytes: u64) {
        saturating_add(&self.allocated_bytes, new_bytes);
        saturating_add(&self.deallocated_bytes, old_bytes);
        let previous = self
            .current_live_bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_sub(old_bytes).saturating_add(new_bytes))
            })
            .unwrap_or_else(|current| current);
        let live = previous.saturating_sub(old_bytes).saturating_add(new_bytes);
        update_peak(&self.peak_live_bytes, live);
    }
}

#[cfg(feature = "snapshot-experiment-alloc")]
fn saturating_add(counter: &AtomicU64, amount: u64) -> u64 {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(amount))
        })
        .unwrap_or_else(|current| current)
}

#[cfg(feature = "snapshot-experiment-alloc")]
fn saturating_sub(counter: &AtomicU64, amount: u64) -> u64 {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_sub(amount))
        })
        .unwrap_or_else(|current| current)
}

#[cfg(feature = "snapshot-experiment-alloc")]
fn update_peak(peak: &AtomicU64, candidate: u64) {
    let _ = peak.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        (candidate > current).then_some(candidate)
    });
}

#[cfg(all(test, feature = "snapshot-experiment-alloc"))]
mod tests {
    use super::*;

    #[test]
    fn tracks_live_and_peak_bytes_across_allocation_reallocation_and_deallocation() {
        let counters = AllocationCounters::new();

        counters.record_allocation_call();
        counters.record_allocation(10);
        counters.record_reallocation_call();
        counters.record_reallocation(10, 25);
        counters.record_deallocation_call();
        counters.record_deallocation(25);

        assert_eq!(
            counters.snapshot(),
            HbkSnapshotExperimentAllocationSnapshot {
                allocation_calls: 1,
                reallocation_calls: 1,
                deallocation_calls: 1,
                allocated_bytes: 35,
                deallocated_bytes: 35,
                current_live_bytes: 0,
                peak_live_bytes: 25,
            }
        );
    }

    #[test]
    fn counters_and_deltas_saturate_instead_of_wrapping() {
        let counters = AllocationCounters::new();
        counters
            .allocated_bytes
            .store(u64::MAX - 1, Ordering::Relaxed);
        counters
            .current_live_bytes
            .store(u64::MAX - 1, Ordering::Relaxed);
        counters.record_allocation(10);
        assert_eq!(counters.allocated_bytes.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(
            counters.current_live_bytes.load(Ordering::Relaxed),
            u64::MAX
        );
        assert_eq!(counters.peak_live_bytes.load(Ordering::Relaxed), u64::MAX);

        let earlier = HbkSnapshotExperimentAllocationSnapshot {
            allocation_calls: 10,
            peak_live_bytes: 100,
            ..HbkSnapshotExperimentAllocationSnapshot::default()
        };
        let later = HbkSnapshotExperimentAllocationSnapshot {
            allocation_calls: 2,
            peak_live_bytes: 90,
            ..HbkSnapshotExperimentAllocationSnapshot::default()
        };
        let delta = later.delta_since(earlier);
        assert_eq!(delta.allocation_calls, 0);
        assert_eq!(delta.peak_live_bytes_growth, 0);
    }
}
