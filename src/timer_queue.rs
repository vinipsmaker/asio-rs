// TODO:
//
// - Timeout operations need to be cancelable.
//   - Create an I/O object to encapsulate each timeout operation?
// - Take a `Duration`.
pub trait TimerQueue {
    /// Always return immediately.
    fn schedule_timeout<F>(&self, timeout_ms: u64, f: F)
        where F : FnOnce() + 'static;
}
