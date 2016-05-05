// TODO:
//
// - move away from callbacks
//   - use coroutines
/// An executor is to function execution as an allocator is to allocation
pub trait Executor {
    /// submit the function for later execution; never block the calling thread
    /// to wait for the function to complete
    fn post<F : FnOnce()>(&self, f: F);
}
