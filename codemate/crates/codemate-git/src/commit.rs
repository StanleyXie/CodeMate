//! Commit information structures.

use chrono::{DateTime, Utc, TimeZone};
use git2::Commit;

/// Information about a git commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Full commit hash (40 hex characters).
    pub hash: String,
    /// Short commit hash (7 characters).
    pub short_hash: String,
    /// Commit message summary (first line).
    pub summary: String,
    /// Full commit message.
    pub message: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Commit timestamp.
    pub timestamp: DateTime<Utc>,
}

impl CommitInfo {
    /// Create from a git2 commit.
    pub fn from_commit(commit: &Commit) -> Self {
        let hash = commit.id().to_string();
        let short_hash = hash[..7.min(hash.len())].to_string();
        
        let summary = commit.summary().unwrap_or("").to_string();
        let message = commit.message().unwrap_or("").to_string();
        
        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        
        let time = commit.time();
        let timestamp = Utc.timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        Self {
            hash,
            short_hash,
            summary,
            message,
            author_name,
            author_email,
            timestamp,
        }
    }

    /// Get author in "Name <email>" format.
    pub fn author(&self) -> String {
        if self.author_email.is_empty() {
            self.author_name.clone()
        } else {
            format!("{} <{}>", self.author_name, self.author_email)
        }
    }
}
