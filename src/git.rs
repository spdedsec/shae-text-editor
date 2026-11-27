//! Git gutter: per-line change markers (added / modified / deleted) from libgit2.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterMark {
    Added,
    Modified,
    Deleted,
}

/// Per-line gutter marks; index is 0-based line number.
#[derive(Debug, Default)]
pub struct GitGutter {
    pub marks: Vec<Option<GutterMark>>,
    pub has_repo: bool,
}

impl GitGutter {
    pub fn empty() -> Self {
        GitGutter { marks: Vec::new(), has_repo: false }
    }

    /// Compute gutter marks for `path` against the repository HEAD.
    pub fn compute(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return GitGutter::empty();
        };
        let Ok(repo) = git2::Repository::discover(path) else {
            return GitGutter::empty();
        };
        let workdir = match repo.workdir() {
            Some(w) => w.to_path_buf(),
            None => return GitGutter::empty(),
        };
        let _rel = match path.strip_prefix(&workdir) {
            Ok(r) => r.to_path_buf(),
            Err(_) => return GitGutter::empty(),
        };

        let head_tree = match head_tree(&repo) {
            Some(t) => t,
            None => {
                // No commits yet: every line is "added".
                let n = count_lines(path);
                return GitGutter {
                    marks: vec![Some(GutterMark::Added); n],
                    has_repo: true,
                };
            }
        };

        let mut marks: Vec<Option<GutterMark>> = Vec::new();
        let diff = repo
            .diff_tree_to_workdir_with_index(Some(&head_tree), None)
            .or_else(|_| repo.diff_tree_to_workdir(Some(&head_tree), None));

        if let Ok(diff) = diff {
            let _ = diff.foreach(
                &mut |_, _| true,
                None,
                Some(&mut |_, hunk: git2::DiffHunk<'_>| {
                    let new_start = hunk.new_start();
                    let new_lines = hunk.new_lines();
                    for i in 0..new_lines {
                        let line_no = new_start + i;
                        if line_no > 0 {
                            ensure(&mut marks, (line_no - 1) as usize);
                            marks[(line_no - 1) as usize] = Some(GutterMark::Modified);
                        }
                    }
                    true
                }),
                Some(&mut |_, _, _| true),
            );
        }

        // Deleted lines are represented as markers on the following line; approximate by
        // leaving them as Modified markers already set. Trim to actual line count.
        let n = count_lines(path);
        marks.resize(n, None);
        GitGutter { marks, has_repo: true }
    }

    pub fn mark_at(&self, line: usize) -> Option<GutterMark> {
        self.marks.get(line).copied().flatten()
    }
}

fn ensure(v: &mut Vec<Option<GutterMark>>, idx: usize) {
    if idx >= v.len() {
        v.resize(idx + 1, None);
    }
}

fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count().max(1))
        .unwrap_or(1)
}

fn head_tree(repo: &git2::Repository) -> Option<git2::Tree<'_>> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    commit.tree().ok()
}