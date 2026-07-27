mod helpers;

#[cfg(debug_assertions)]
mod dev;
#[cfg(not(debug_assertions))]
mod release;

#[cfg(debug_assertions)]
pub use dev::{TraceSpan, event, init, snapshot, span};
#[cfg(not(debug_assertions))]
pub use release::{TraceSpan, event, init, snapshot, span};
