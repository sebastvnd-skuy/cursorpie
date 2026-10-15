//! Giving freed memory back to macOS.

use std::ffi::c_void;

unsafe extern "C" {
    /// `size_t malloc_zone_pressure_relief(malloc_zone_t *zone, size_t goal)` from libmalloc.
    fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
}

/// Asks the allocator to return free pages to the system and reports how many bytes it released.
///
/// Dropping a texture or a decoded image frees it for the *process*, but the allocator usually
/// keeps the pages, so the app still shows up as using the memory in Activity Monitor. Call this
/// after freeing something big (the UI does so when it goes to sleep, see
/// [`ui::sleep`](crate::ui::sleep)). It never touches memory that is in use.
pub fn release_unused_memory() -> usize {
    // SAFETY: a null zone means "every zone" and a goal of 0 means "as much as possible". No
    // pointer is read or written.
    unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) }
}
