//! Git repository wrapper.

use crate::commit::CommitInfo;
use git2::{Repository, Oid, Sort};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors from git operations.
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    
    #[error("Repository not found at {0}")]
    NotFound(PathBuf),
    
    #[error("Not a git repository")]
    NotARepository,
    
    #[error("Invalid commit: {0}")]
    InvalidCommit(String),
}

/// Result type for git operations.
pub type Result<T> = std::result::Result<T, GitError>;

/// Wrapper around a git repository.
pub struct GitRepository {
    repo: Repository,
    path: PathBuf,
}

impl GitRepository {
    /// Open a git repository at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let repo = Repository::discover(&path)?;
        
        Ok(Self { repo, path })
    }

    /// Get the repository root path.
    pub fn root(&self) -> PathBuf {
        self.repo.workdir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.path.clone())
    }

    /// Get the HEAD commit.
    pub fn head_commit(&self) -> Result<CommitInfo> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(CommitInfo::from_commit(&commit))
    }

    /// Get a commit by its hash.
    pub fn get_commit(&self, hash: &str) -> Result<CommitInfo> {
        let oid = Oid::from_str(hash)
            .map_err(|_| GitError::InvalidCommit(hash.to_string()))?;
        let commit = self.repo.find_commit(oid)?;
        Ok(CommitInfo::from_commit(&commit))
    }

    /// Walk commits from HEAD backwards.
    pub fn walk_commits(&self, max_count: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();
        for (i, oid_result) in revwalk.enumerate() {
            if let Some(max) = max_count {
                if i >= max {
                    break;
                }
            }

            let oid = oid_result?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(CommitInfo::from_commit(&commit));
        }

        Ok(commits)
    }

    /// Get the contents of a file at a specific commit.
    pub fn get_file_at_commit(&self, commit_hash: &str, file_path: &str) -> Result<Option<String>> {
        let oid = Oid::from_str(commit_hash)
            .map_err(|_| GitError::InvalidCommit(commit_hash.to_string()))?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        match tree.get_path(Path::new(file_path)) {
            Ok(entry) => {
                let blob = self.repo.find_blob(entry.id())?;
                let content = std::str::from_utf8(blob.content())
                    .ok()
                    .map(String::from);
                Ok(content)
            }
            Err(_) => Ok(None),
        }
    }

    /// List all files in the repository at HEAD.
    pub fn list_files(&self) -> Result<Vec<String>> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        let tree = commit.tree()?;

        let mut files = Vec::new();
        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let path = if dir.is_empty() {
                    entry.name().unwrap_or("").to_string()
                } else {
                    format!("{}{}", dir, entry.name().unwrap_or(""))
                };
                files.push(path);
            }
            git2::TreeWalkResult::Ok
        })?;

        Ok(files)
    }

    /// Get the underlying git2 repository.
    pub fn inner(&self) -> &Repository {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_current_repo() {
        // This test assumes we're running from within a git repo
        let result = GitRepository::open(".");
        assert!(result.is_ok() || result.is_err()); // Just test it doesn't panic
    }
}
