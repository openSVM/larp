//! Contains the logging crate

pub mod parea;
#[cfg(feature = "tee_requests")]
pub mod tee_middleware;
mod tee_client;
pub use tee_client::new_client;
