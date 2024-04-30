//! Structured parallel execution for async Rust.
//!
//! > Concurrency is a system-structuring mechanism, parallelism is a resource.
//!
//! This is a replacement for the common `Task` idiom. Rather than providing a
//! separate family of APIs for concurrency and parallelism, this library
//! provides a `ParallelFuture` type. When this type is scheduled concurrently
//! it will provide parallel execution.
//!
//! # Limitations
//!
//! Rust does not yet provide a mechanism for async destructors. That means that
//! on an early return of any kind, Rust can't guarantee that certain
//! asynchronous operations run before others. This is a language-level
//! limitation with no existing workarounds possible. `ParallelFuture` is designed to
//! work with async destructors once they land.
//!
//! `ParallelFuture` starts lazily and does not provide a manual `detach`
//! method. However it can be manually polled once and then passed to
//! `mem::forget`, which will keep the future running on another thread. In the
//! absence of unforgettable types (linear types), Rust cannot `ParallelFuture`s
//! from being unmanaged.
//!
//! # Examples
//!
//! ```
//! use parallel_future::prelude::*;
//! use futures_concurrency::prelude::*;
//!
//! async_std::task::block_on(async {
//!     let a = async { 1 }.par();        // ← returns `ParallelFuture`
//!     let b = async { 2 }.par();        // ← returns `ParallelFuture`
//!
//!     let (a, b) = (a, b).join().await; // ← concurrent `.await`
//!     assert_eq!(a + b, 3);
//! })
//! ```

#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]

use pin_project::{pin_project, pinned_drop};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_std::task;

/// The `parallel-future` prelude.
pub mod prelude {
    pub use super::IntoFutureExt as _;
}

/// A parallelizable future.
///
/// This type is constructed by the [`par`][crate::IntoFutureExt::par] method on [`IntoFutureExt`][crate::IntoFutureExt].
///
/// # Examples
///
/// ```
/// use parallel_future::prelude::*;
/// use futures_concurrency::prelude::*;
///
/// async_std::task::block_on(async {
///     let a = async { 1 }.par();        // ← returns `ParallelFuture`
///     let b = async { 2 }.par();        // ← returns `ParallelFuture`
///
///     let (a, b) = (a, b).join().await; // ← concurrent `.await`
///     assert_eq!(a + b, 3);
/// })
/// ```
#[derive(Debug)]
#[pin_project(PinnedDrop)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ParallelFuture<Fut: Future> {
    #[pin]
    handle: Option<task::JoinHandle<Fut::Output>>,
}

impl<Fut> Future for ParallelFuture<Fut>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    type Output = <Fut as Future>::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        Pin::new(&mut this.handle.as_pin_mut().unwrap()).poll(cx)
    }
}

/// Cancel the `ParallelFuture` when dropped.
#[pinned_drop]
impl<Fut: Future> PinnedDrop for ParallelFuture<Fut> {
    fn drop(self: Pin<&mut Self>) {
        let mut this = self.project();
        let handle = this.handle.take().unwrap();
        let _ = handle.cancel();
    }
}

/// Extend the `Future` trait.
pub trait IntoFutureExt: IntoFuture + Sized
where
    <Self as IntoFuture>::IntoFuture: Send + 'static,
    <Self as IntoFuture>::Output: Send + 'static,
{
    /// Convert this future into a parallelizable future.
    ///
    /// # Examples
    ///
    /// ```
    /// use parallel_future::prelude::*;
    /// use futures_concurrency::prelude::*;
    ///
    /// async_std::task::block_on(async {
    ///     let a = async { 1 }.par();        // ← returns `ParallelFuture`
    ///     let b = async { 2 }.par();        // ← returns `ParallelFuture`
    ///
    ///     let (a, b) = (a, b).join().await; // ← concurrent `.await`
    ///     assert_eq!(a + b, 3);
    /// })
    /// ```
    fn par(self) -> ParallelFuture<<Self as IntoFuture>::IntoFuture> {
        ParallelFuture {
            handle: Some(task::spawn(self.into_future())),
        }
    }
}

impl<Fut> IntoFutureExt for Fut
where
    Fut: IntoFuture,
    <Fut as IntoFuture>::IntoFuture: Send + 'static,
    <Fut as IntoFuture>::Output: Send + 'static,
{
}

#[cfg(test)]
mod test {
    use super::prelude::*;

    #[test]
    fn spawn() {
        async_std::task::block_on(async {
            let res = async { "nori is a horse" }.par().await;
            assert_eq!(res, "nori is a horse");
        })
    }
}
