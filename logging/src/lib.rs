//! Contains the logging crate

pub mod parea;
mod tee_client;
#[cfg(feature = "tee_requests")]
pub mod tee_middleware;
pub use tee_client::new_client;
