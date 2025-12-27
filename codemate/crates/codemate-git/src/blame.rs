//! Git blame functionality.

use crate::repository::{GitRepository, Result};
use chrono::{DateTime, Utc, TimeZone};
use git2::BlameOptions;

/// Blame information for a line of code.
#[derive(Debug, Clone)]
pub struct BlameInfo {
    /// Commit hash that last modified this line.
    pub commit_hash: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// When the line was last modified.
    pub timestamp: DateTime<Utc>,
    /// Original line number in the commit.
    pub original_line: usize,
    /// Current line number.
    pub final_line: usize,
}

impl BlameInfo {
    /// Get author in "Name <email>" format.
    pub fn author(&self) -> String {
        if self.author_email.is_empty() {
            self.author_name.clone()
        } else {
            format!("{} <{}>", self.author_name, self.author_email)
        }
    }
}

impl GitRepository {
    /// Get blame information for a file.
    pub fn blame_file(&self, file_path: &str) -> Result<Vec<BlameInfo>> {
        let mut opts = BlameOptions::new();
        let blame = self.inner().blame_file(std::path::Path::new(file_path), Some(&mut opts))?;

        let mut infos = Vec::new();
        for hunk in blame.iter() {
            let sig = hunk.final_signature();
            let time = sig.when();
            let timestamp = Utc.timestamp_opt(time.seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            let info = BlameInfo {
                commit_hash: hunk.final_commit_id().to_string(),
                author_name: sig.name().unwrap_or("Unknown").to_string(),
                author_email: sig.email().unwrap_or("").to_string(),
                timestamp,
                original_line: hunk.orig_start_line(),
                final_line: hunk.final_start_line(),
            };
            infos.push(info);
        }

        Ok(infos)
    }

    /// Get blame for a specific line range.
    pub fn blame_lines(&self, file_path: &str, start_line: usize, end_line: usize) -> Result<Vec<BlameInfo>> {
        let all_blame = self.blame_file(file_path)?;
        
        Ok(all_blame.into_iter()
            .filter(|b| b.final_line >= start_line && b.final_line <= end_line)
            .collect())
    }

    /// Get the primary author for a line range (most lines attributed to them).
    pub fn primary_author(&self, file_path: &str, start_line: usize, end_line: usize) -> Result<Option<BlameInfo>> {
        let blame = self.blame_lines(file_path, start_line, end_line)?;
        
        if blame.is_empty() {
            return Ok(None);
        }

        // Count lines per author
        use std::collections::HashMap;
        let mut author_counts: HashMap<String, (usize, BlameInfo)> = HashMap::new();
        
        for info in blame {
            let key = info.author();
            author_counts
                .entry(key)
                .and_modify(|(count, _)| *count += 1)
                .or_insert((1, info));
        }

        // Find author with most lines
        let primary = author_counts.into_values()
            .max_by_key(|(count, _)| *count)
            .map(|(_, info)| info);

        Ok(primary)
    }
}
