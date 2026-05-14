//! Diagnostics, error, limit, and test-support APIs.

pub mod performance;
pub mod report;

pub use crate::{debug, errors, limits, testing};
pub use performance::*;
pub use report::*;
