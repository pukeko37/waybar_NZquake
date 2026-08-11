//! Domain layer: pure value types and business rules for earthquake data.
//! No I/O, no serialization, no knowledge of any adapter.

pub mod error;
pub mod models;
pub mod types;

pub use error::*;
pub use models::*;
pub use types::*;
