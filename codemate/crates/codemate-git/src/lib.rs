//! CodeMate Git Integration
//!
//! Provides git-aware indexing with commit tracking and blame attribution.

pub mod repository;
pub mod blame;
pub mod commit;

pub use repository::GitRepository;
pub use blame::BlameInfo;
pub use commit::CommitInfo;
