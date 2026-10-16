//! Monitor scale: converts between the physical pixels Win32 reports and the logical points the
//! rest of CursorPie (and egui) uses.

use super::util::scale_from_dpi;
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromPoint};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// Bits of the scale used for the last pointer event (an `f64`; 1.0 at start). The hook stores it
/// before dispatching, so a lookup that needs physical pixels again (the window under the pointer)
/// uses the same scale as the event it belongs to.
static LAST_SCALE: AtomicU64 = AtomicU64::new(0x3FF0_0000_0000_0000);

/// Scale of the monitor containing the physical point `(x, y)` (nearest monitor when outside).
pub(crate) fn scale_at_physical(x: i32, y: i32) -> f64 {
    // SAFETY: plain queries; out-pointers are valid for the call.
    unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let (mut dx, mut dy) = (0u32, 0u32);
        if GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) == 0 {
            scale_from_dpi(dx)
        } else {
            1.0
        }
    }
}

/// Remembers `scale` as the one of the current pointer event.
pub(crate) fn set_last_scale(scale: f64) {
    LAST_SCALE.store(scale.to_bits(), Ordering::Relaxed);
}

/// The scale stored by [`set_last_scale`].
pub(crate) fn last_scale() -> f64 {
    f64::from_bits(LAST_SCALE.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_scale_round_trips() {
        let before = last_scale();
        set_last_scale(1.75);
        assert_eq!(last_scale(), 1.75);
        set_last_scale(before);
    }

    #[test]
    fn the_initial_scale_is_one() {
        assert_eq!(f64::from_bits(0x3FF0_0000_0000_0000), 1.0);
    }
}
