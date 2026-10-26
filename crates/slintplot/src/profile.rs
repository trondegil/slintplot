//! Timing of the layout computations, for diagnostics and stress testing.

use std::cell::Cell;
use std::time::Duration;

thread_local! {
    static LAST_LAYOUT: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    static TOTAL: Cell<(u32, Duration)> = const { Cell::new((0, Duration::ZERO)) };
}

/// How long the most recent chart layout took, including reading the models.
pub fn last_layout_time() -> Duration {
    LAST_LAYOUT.with(Cell::get)
}

/// Returns the number of layout and axis computations, and their total time,
/// since the previous call.
pub fn take_totals() -> (u32, Duration) {
    TOTAL.with(|t| t.replace((0, Duration::ZERO)))
}

/// A started timing. In the browser (`wasm32-unknown-unknown`), which has no
/// clock in the standard library, timings are recorded as zero.
#[doc(hidden)]
pub struct Started(#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))] std::time::Instant);

#[doc(hidden)]
pub fn start() -> Started {
    Started(
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        std::time::Instant::now(),
    )
}

impl Started {
    pub fn elapsed(&self) -> Duration {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        return self.0.elapsed();
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        return Duration::ZERO;
    }
}

#[doc(hidden)]
pub fn record_layout(duration: Duration) {
    LAST_LAYOUT.with(|last| last.set(duration));
    record(duration);
}

#[doc(hidden)]
pub fn record(duration: Duration) {
    TOTAL.with(|t| {
        let (calls, total) = t.get();
        t.set((calls + 1, total + duration));
    });
}
