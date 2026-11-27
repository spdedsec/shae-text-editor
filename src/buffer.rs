//! Text document: a rope-backed buffer with dirty tracking, file path and language.

use ropey::Rope;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug)]
pub struct Document {
    pub rope: Rope,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub line_ending: LineEnding,
    /// Monotonic id so tabs can be tracked even with duplicate paths.
    pub id: u64,
    /// Last saved content hash for dirty detection on reload.
    pub version: u64,
}

impl Document {
    pub fn new(id: u64, text: String) -> Self {
        let line_ending = if text.contains("\r\n") {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        };
        Document {
            rope: Rope::from_str(&text),
            path: None,
            dirty: false,
            line_ending,
            id,
            version: 0,
        }
    }

    pub fn from_file(id: u64, path: PathBuf) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(&path)?;
        let line_ending = if text.contains("\r\n") {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        };
        Ok(Document {
            rope: Rope::from_str(&text),
            path: Some(path),
            dirty: false,
            line_ending,
            id,
            version: 0,
        })
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, idx: usize) -> ropey::RopeSlice<'_> {
        self.rope.line(idx)
    }

    /// Full text as a String.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        self.rope.get_char(char_idx)
    }

    /// Convert a (line, char_col) to a global char index.
    pub fn char_index(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        // line includes trailing newline; clamp col within visible chars.
        let content_len = line_len.saturating_sub(if line_len > 0 && self.rope.line(line).char(line_len - 1) == '\n' { 1 } else { 0 });
        let col = col.min(content_len);
        line_start + col
    }

    /// Convert a global char index to (line, char_col).
    pub fn pos_of(&self, char_idx: usize) -> (usize, usize) {
        let char_idx = char_idx.min(self.len_chars());
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        (line, char_idx - line_start)
    }

    /// Character column (in chars) of a line, excluding the newline.
    pub fn line_char_len(&self, line: usize) -> usize {
        let l = self.rope.line(line);
        let n = l.len_chars();
        if n > 0 && l.char(n - 1) == '\n' { n - 1 } else { n }
    }

    /// Insert text at a char index, normalizing line endings.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let char_idx = char_idx.min(self.len_chars());
        let normalized = self.normalize_newlines(text);
        self.rope.insert(char_idx, &normalized);
        self.touch();
    }

    /// Delete `count` chars starting at `char_idx`.
    pub fn delete(&mut self, char_idx: usize, count: usize) {
        if count == 0 {
            return;
        }
        let start = char_idx.min(self.len_chars());
        let end = (start + count).min(self.len_chars());
        if end > start {
            self.rope.remove(start..end);
            self.touch();
        }
    }

    fn normalize_newlines(&self, text: &str) -> String {
        match self.line_ending {
            LineEnding::Lf => text.replace("\r\n", "\n"),
            LineEnding::Crlf => text.replace("\r\n", "\n").replace('\n', "\r\n"),
        }
    }

    /// Replace the entire buffer.
    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.touch();
    }

    fn touch(&mut self) {
        self.dirty = true;
        self.version += 1;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        let path = self.path.clone().expect("save requires a path");
        let text = self.text();
        std::fs::write(&path, text.as_bytes())?;
        self.dirty = false;
        Ok(())
    }

    pub fn display_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}