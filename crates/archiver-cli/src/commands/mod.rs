//! Command implementations

mod index;
mod search;
mod generate;
mod stats;

pub use index::cmd_index;
pub use search::cmd_search;
pub use generate::cmd_generate;
pub use stats::cmd_stats;
