//! Codex source reader.

mod rate_limits;
pub mod record_parser;
pub mod record_reader;
mod session_index;

pub use record_reader::CodexReader;
