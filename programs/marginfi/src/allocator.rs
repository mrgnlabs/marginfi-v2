/*
================================================================================
MARGINFI CUSTOM BUMP HEAP ALLOCATOR
================================================================================

This module provides a custom memory allocator optimized for Marginfi's risk
engine operations, specifically enabling support for up to 16 positions without
requiring `requestHeapFrame` for typical operations.

================================================================================
COMPARISON: NATIVE SOLANA VS MARGINFI ALLOCATOR
================================================================================

┌─────────────────────┬────────────────────────┬────────────────────────────────┐
│ Feature             │ Native Solana          │ Marginfi                       │
├─────────────────────┼────────────────────────┼────────────────────────────────┤
│ Heap Size           │ 32 KiB                 │ 256 KiB (8 × 32 KiB)           │
│ Direction           │ Backward (top-down)    │ Forward (bottom-up)            │
│ Start Address       │ HEAP_START + HEAP_LEN  │ HEAP_START + sizeof(ptr)       │
│ Position Storage    │ Internal struct        │ At HEAP_START (first 8 bytes)  │
│ Heap Reuse          │ Not supported          │ heap_pos() / heap_restore()    │
│ requestHeapFrame    │ Required for >32 KiB   │ Only for >32 KiB operations    │
└─────────────────────┴────────────────────────┴────────────────────────────────┘

## Native Solana Allocator (solana_program::entrypoint)

The default Solana bump allocator works as follows:

```
HEAP_START_ADDRESS: 0x300000000
HEAP_LENGTH: 32 * 1024 (32 KiB)

Allocation Direction: BACKWARD (High → Low)
─────────────────────────────────────────────

0x300000000           0x300008000
      │                    │
      ▼                    ▼
      [====================]
                          ▲
                          │
                 pos starts here, moves LEFT with each alloc

Code (simplified):
    pos = self.start + self.len;        // Start at top (0x300008000)
    pos = pos.saturating_sub(size);     // Move down
    pos &= !(align - 1);                // Align
```

## Why Backward Allocation Breaks with Extended Heap

If we simply compile program with increased HEAP_LENGTH without changing direction
and or not using `requestHeapFrame` (for functions that fit in 32 KiB), we get this situation:


```
0x300000000           0x300008000                          0x300040000
      │                    │                                     │
      ▼                    ▼                                     ▼
      [--------------------│-------------------------------------]
      ^                    ^                                     ^
      │                    │                                     │
 VM Lower              VM Upper                           pos starts HERE!
 Boundary              Boundary                           ACCESS VIOLATION!
 (always valid)        (without requestHeapFrame)
```

The allocator tries to start at 0x300040000, which is OUTSIDE the default 32 KiB
VM-accessible region (0x300000000 - 0x300008000), causing immediate access violation.

## Marginfi Solution: Forward Allocation

By allocating FORWARD from the bottom, we guarantee safety:

```
0x300000000           0x300008000                          0x300040000
      │                    │                                     │
      ▼                    ▼                                     ▼
      [--------------------│-------------------------------------]
      ▲                    ▲                                     ▲
      │                    │                                     │
 pos starts HERE      32 KiB boundary                    256 KiB boundary
 (ALWAYS SAFE!)       (no requestHeapFrame)              (with requestHeapFrame)

Code (this allocator):
    pos = HEAP_START + sizeof(ptr);     // Start at bottom (after reserved)
    pos = align_up(pos, align);         // Align upward
    pos = pos + size;                   // Move up
```

================================================================================
KEY IMPROVEMENTS OVER NATIVE SOLANA ALLOCATOR
================================================================================

1. GRACEFUL DEGRADATION
   - Operations using ≤32 KiB work without requestHeapFrame
   - Operations needing >32 KiB automatically work when requestHeapFrame is used
   - No code changes needed between the two modes

2. HEAP POSITION TRACKING
   - Position pointer stored at HEAP_START_ADDRESS (first 8 bytes)
   - Enables runtime heap introspection via heap_pos()
   - Enables temporary allocation patterns via heap_restore()

3. MEMORY RECYCLING (Critical for Risk Engine)
   - Native Solana: No way to reclaim memory mid-execution
   - Marginfi: heap_restore() allows "freeing" temporary allocations

   Without recycling (16 positions × ~3 KiB each = 48+ KiB):
   ┌────────┬────────┬────────┬─────────────────┬────────┐
   │ Pos 1  │ Pos 2  │ Pos 3  │ ... 48+ KiB ... │ Pos 16 │  EXCEEDS 32 KiB!
   └────────┴────────┴────────┴─────────────────┴────────┘

   With recycling (same memory reused, peak ~3-4 KiB):
   ┌────────┐     ┌────────┐           ┌────────┐
   │ Pos 1  │ ──▶ │ Pos 2  │ ──▶ ... ─▶│ Pos 16 │  FITS IN 32 KiB!
   └────────┘     └────────┘           └────────┘
       ▼              ▼                     ▼
    restore()     restore()            restore()

================================================================================
SAFETY REQUIREMENTS FOR heap_restore()
================================================================================

⚠️  CRITICAL: heap_restore() does NOT run destructors. It simply resets the
    allocation pointer, making the memory available for reuse.

✓ SAFE patterns:
  - Extract primitive values (I80F48, u64, bool, [u8; N]) to stack before restore
  - Use Copy types that don't need heap cleanup
  - Drop all Boxes/Vecs/Strings naturally before restore

✗ UNSAFE patterns:
  - Holding references to heap data across restore boundary
  - Accessing Box/Vec/String contents after restore
  - Storing heap pointers in struct fields that survive restore

Example of SAFE usage:
```rust
let checkpoint = heap_pos();
{
    let temp_vec: Vec<u8> = vec![1, 2, 3];          // Heap allocation
    let sum: u64 = temp_vec.iter().sum();           // Copy to stack!
    // temp_vec goes out of scope here (but dealloc is no-op anyway)
}
heap_restore(checkpoint);
// sum is valid (stack), temp_vec's memory is now available for reuse
```

================================================================================
REFERENCES
================================================================================

Native Solana allocator source:
  - https://github.com/solana-labs/solana/blob/master/sdk/program/src/entrypoint.rs

Solana custom heap example:
  - https://github.com/solana-labs/solana-program-library/blob/master/examples/rust/custom-heap/src/entrypoint.rs

*/

#[cfg(target_os = "solana")]
use anchor_lang::solana_program::entrypoint::HEAP_START_ADDRESS;
#[cfg(target_os = "solana")]
use std::{alloc::Layout, mem::size_of, ptr::null_mut};
#[cfg(target_os = "solana")]
pub const HEAP_LENGTH: usize = 8 * 32 * 1024;
#[cfg(target_os = "solana")]
pub struct BumpAllocator;

#[cfg(target_os = "solana")]
unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        const POS_PTR: *mut usize = HEAP_START_ADDRESS as *mut usize;
        const TOP_ADDRESS: usize = HEAP_START_ADDRESS as usize + HEAP_LENGTH;
        const BOTTOM_ADDRESS: usize = HEAP_START_ADDRESS as usize + size_of::<*mut u8>();

        let mut pos = *POS_PTR;
        if pos == 0 {
            // First time, set starting position to bottom address
            pos = BOTTOM_ADDRESS;
        }

        pos = (pos + layout.align() - 1) & !(layout.align() - 1);

        let next_pos = pos.saturating_add(layout.size());

        // Check if we've exceeded the heap limit
        // Note: This is our configured limit (256 KiB), but the VM boundary
        // may be smaller (32 KiB default) without requestHeapFrame
        if next_pos > TOP_ADDRESS {
            return null_mut();
        }

        // Update position pointer and return allocated address
        *POS_PTR = next_pos;
        pos as *mut u8
    }

    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

#[cfg(all(
    target_os = "solana",
    feature = "custom-heap",
    not(feature = "no-entrypoint")
))]
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

/// Returns the current heap position for creating a checkpoint.
#[cfg(target_os = "solana")]
#[inline]
pub fn heap_pos() -> usize {
    unsafe {
        let pos_ptr = HEAP_START_ADDRESS as *mut usize;
        let pos = *pos_ptr;
        if pos == 0 {
            // Heap hasn't been used yet, return bottom address
            HEAP_START_ADDRESS as usize + size_of::<*mut u8>()
        } else {
            pos
        }
    }
}

/// Restores the heap position to a previously saved checkpoint.
#[cfg(target_os = "solana")]
#[inline]
pub fn heap_restore(pos: usize) {
    unsafe {
        let pos_ptr = HEAP_START_ADDRESS as *mut usize;
        *pos_ptr = pos;
    }
}

#[cfg(not(target_os = "solana"))]
#[inline]
pub fn heap_pos() -> usize {
    0
}

#[cfg(not(target_os = "solana"))]
#[inline]
pub fn heap_restore(_pos: usize) {}
