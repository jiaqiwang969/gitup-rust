use anyhow::Result;
use git2::{Delta, DiffOptions, Repository as Git2Repository};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    TypeChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub origin: LineOrigin,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineOrigin {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file: DiffFile,
    pub hunks: Vec<DiffHunk>,
    pub lines: Vec<DiffLine>,
    pub binary: bool,
}

pub struct Diff<'repo> {
    repo: &'repo Git2Repository,
}

impl<'repo> Diff<'repo> {
    pub fn new(repo: &'repo Git2Repository) -> Self {
        Diff { repo }
    }

    /// Get diff between working directory and index
    pub fn workdir_to_index(&self) -> Result<Vec<FileDiff>> {
        let index = self.repo.index()?;
        let diff = self.repo.diff_index_to_workdir(Some(&index), None)?;
        self.process_diff(diff)
    }

    /// Get diff between index and HEAD
    pub fn index_to_head(&self) -> Result<Vec<FileDiff>> {
        let head = self.repo.head()?.peel_to_tree()?;
        let index = self.repo.index()?;
        let diff = self.repo.diff_tree_to_index(Some(&head), Some(&index), None)?;
        self.process_diff(diff)
    }

    /// Get diff between two commits
    pub fn between_commits(&self, old: &str, new: &str) -> Result<Vec<FileDiff>> {
        let old_commit = self.repo.find_commit(git2::Oid::from_str(old)?)?;
        let new_commit = self.repo.find_commit(git2::Oid::from_str(new)?)?;

        let old_tree = old_commit.tree()?;
        let new_tree = new_commit.tree()?;

        let diff = self.repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
        self.process_diff(diff)
    }

    /// Get diff for a specific commit
    pub fn for_commit(&self, commit_id: &str) -> Result<Vec<FileDiff>> {
        let commit = self.repo.find_commit(git2::Oid::from_str(commit_id)?)?;
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = self.repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        self.process_diff(diff)
    }

    /// Get diff for a specific file in working directory
    pub fn file_diff(&self, path: &Path) -> Result<FileDiff> {
        // First try to get diff between index and working directory
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        let mut file_diffs = self.process_diff(diff)?;

        if !file_diffs.is_empty() {
            // Found changes in working directory
            return file_diffs.pop().ok_or_else(|| anyhow::anyhow!("No diff found for file: {:?}", path));
        }

        // If no working directory changes, check if file is staged
        let head = self.repo.head()?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_index(Some(&head), None, Some(&mut opts))?;
        let mut file_diffs = self.process_diff(diff)?;

        file_diffs.pop().ok_or_else(|| anyhow::anyhow!("No diff found for file: {:?}", path))
    }

    /// Get staged diff for a specific file
    pub fn staged_file_diff(&self, path: &Path) -> Result<FileDiff> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path);

        // Try to get diff from HEAD to index
        let diff = if let Ok(head) = self.repo.head() {
            let head_tree = head.peel_to_tree()?;
            self.repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        } else {
            // No HEAD (initial commit), compare against empty tree
            self.repo.diff_tree_to_index(None, None, Some(&mut opts))?
        };

        let mut file_diffs = self.process_diff(diff)?;

        file_diffs.pop().ok_or_else(|| anyhow::anyhow!("No staged diff found for file: {:?}", path))
    }

    fn process_diff(&self, mut diff: git2::Diff) -> Result<Vec<FileDiff>> {
        let mut file_diffs = Vec::new();

        // First, find similar files (for renames/copies)
        diff.find_similar(None)?;

        let stats = diff.stats()?;
        let num_deltas = stats.files_changed();

        for idx in 0..num_deltas {
            if let Some(delta) = diff.get_delta(idx) {
                let file = DiffFile {
                    path: delta.new_file().path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    old_path: delta.old_file().path()
                        .map(|p| p.to_string_lossy().into_owned()),
                    status: Self::convert_status(delta.status()),
                };

                let mut file_diff = FileDiff {
                    file,
                    hunks: Vec::new(),
                    lines: Vec::new(),
                    binary: delta.new_file().is_binary() || delta.old_file().is_binary(),
                };

                // Get patch for this file using Patch::from_diff
                if let Ok(Some(patch)) = git2::Patch::from_diff(&diff, idx) {
                    // Process hunks and lines
                    let num_hunks = patch.num_hunks();
                    for hunk_idx in 0..num_hunks {
                        if let Ok((hunk, num_lines)) = patch.hunk(hunk_idx) {
                            file_diff.hunks.push(DiffHunk {
                                old_start: hunk.old_start(),
                                old_lines: hunk.old_lines(),
                                new_start: hunk.new_start(),
                                new_lines: hunk.new_lines(),
                                header: String::from_utf8_lossy(hunk.header()).into_owned(),
                            });

                            for line_idx in 0..num_lines {
                                if let Ok(line) = patch.line_in_hunk(hunk_idx, line_idx) {
                                    let origin = match line.origin() {
                                        '+' => LineOrigin::Addition,
                                        '-' => LineOrigin::Deletion,
                                        _ => LineOrigin::Context,
                                    };

                                    file_diff.lines.push(DiffLine {
                                        origin,
                                        content: String::from_utf8_lossy(line.content()).into_owned(),
                                        old_lineno: line.old_lineno(),
                                        new_lineno: line.new_lineno(),
                                    });
                                }
                            }
                        }
                    }
                }

                file_diffs.push(file_diff);
            }
        }

        Ok(file_diffs)
    }

    fn convert_status(status: Delta) -> FileStatus {
        match status {
            Delta::Added => FileStatus::Added,
            Delta::Deleted => FileStatus::Deleted,
            Delta::Modified => FileStatus::Modified,
            Delta::Renamed => FileStatus::Renamed,
            Delta::Copied => FileStatus::Copied,
            Delta::Untracked => FileStatus::Untracked,
            Delta::Ignored => FileStatus::Ignored,
            Delta::Typechange => FileStatus::TypeChange,
            _ => FileStatus::Modified,
        }
    }
}

/// Get diff statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl DiffStats {
    pub fn from_diffs(diffs: &[FileDiff]) -> Self {
        let mut insertions = 0;
        let mut deletions = 0;

        for diff in diffs {
            for line in &diff.lines {
                match line.origin {
                    LineOrigin::Addition => insertions += 1,
                    LineOrigin::Deletion => deletions += 1,
                    _ => {}
                }
            }
        }

        DiffStats {
            files_changed: diffs.len(),
            insertions,
            deletions,
        }
    }
}