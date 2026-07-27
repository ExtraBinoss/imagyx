mod helpers;

#[cfg(debug_assertions)]
mod dev;
#[cfg(not(debug_assertions))]
mod release;

#[cfg(debug_assertions)]
pub use dev::{event, init, span};
#[cfg(not(debug_assertions))]
pub use release::{event, init, span};
