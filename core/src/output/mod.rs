//! Structured output format for analysis results

mod builder;
mod post_processing;
mod types;

pub use builder::OutputBuilder;
pub use post_processing::{apply_label, apply_top};
pub use types::*;
