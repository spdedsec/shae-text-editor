//! Find and replace state and operations.

use crate::buffer::Document;

#[derive(Debug)]
pub struct FindState {
    pub open: bool,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    /// Match start positions as char indices.
    pub matches: Vec<usize>,
    /// Match lengths in chars, parallel to `matches`.
    pub match_lens: Vec<usize>,
    pub current: usize,
    pub error: Option<String>,
}

impl FindState {
    pub fn new() -> Self {
        FindState {
            open: false,
            query: String::new(),
            replacement: String::new(),
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            matches: Vec::new(),
            match_lens: Vec::new(),
            current: 0,
            error: None,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Recompute all matches in the document.
    pub fn recompute(&mut self, doc: &Document) {
        self.matches.clear();
        self.match_lens.clear();
        self.error = None;
        if self.query.is_empty() {
            self.current = 0;
            return;
        }
        let text = doc.text();
        let result = if self.use_regex {
            regex_find_all(&text, &self.query, self.case_sensitive)
        } else {
            Ok(plain_find_all(&text, &self.query, self.case_sensitive, self.whole_word))
        };
        match result {
            Ok(v) => {
                self.matches = v.iter().map(|(s, _)| *s).collect();
                self.match_lens = v.iter().map(|(_, l)| *l).collect();
            }
            Err(e) => self.error = Some(e),
        }
        if self.current >= self.matches.len() {
            self.current = self.matches.len().saturating_sub(1);
        }
    }

    pub fn current_match(&self) -> Option<(usize, usize)> {
        self.matches
            .get(self.current)
            .map(|s| (*s, self.match_lens.get(self.current).copied().unwrap_or(0)))
    }

    pub fn next(&mut self, count: usize) {
        if count > 0 {
            self.current = (self.current + 1) % count;
        }
    }

    pub fn prev(&mut self, count: usize) {
        if count > 0 {
            self.current = (self.current + count - 1) % count;
        }
    }

    pub fn replace_current(&mut self, doc: &mut Document) -> bool {
        let Some((start, len)) = self.current_match() else {
            return false;
        };
        if len == 0 {
            return false;
        }
        doc.delete(start, len);
        doc.insert(start, &self.replacement);
        true
    }

    pub fn replace_all(&mut self, doc: &mut Document) -> usize {
        self.recompute(doc);
        let pairs: Vec<(usize, usize)> = self
            .matches
            .iter()
            .cloned()
            .zip(self.match_lens.iter().cloned())
            .collect();
        let mut count = 0;
        for (start, len) in pairs.into_iter().rev() {
            if len == 0 {
                continue;
            }
            doc.delete(start, len);
            doc.insert(start, &self.replacement);
            count += 1;
        }
        self.recompute(doc);
        count
    }
}

fn plain_find_all(text: &str, query: &str, case_sensitive: bool, whole_word: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let hay: Vec<char> = text.chars().collect();
    let needle: Vec<char> = if case_sensitive {
        query.chars().collect()
    } else {
        query.to_lowercase().chars().collect()
    };
    if needle.is_empty() {
        return out;
    }
    let n = hay.len();
    let m = needle.len();
    let hay_cmp: Vec<char> = if case_sensitive {
        hay.clone()
    } else {
        text.to_lowercase().chars().collect()
    };
    let mut i = 0;
    while i + m <= n {
        if hay_cmp[i..i + m] == needle[..] && (!whole_word || is_word_boundary(&hay, i, m)) {
            out.push((i, m));
            i += m.max(1);
        } else {
            i += 1;
        }
    }
    out
}

fn is_word_boundary(hay: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word(hay[start - 1]);
    let after = start + len;
    let after_ok = after >= hay.len() || !is_word(hay[after]);
    before_ok && after_ok
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn build_regex(query: &str, case_sensitive: bool) -> Result<regex::Regex, String> {
    let pat = if case_sensitive {
        query.to_string()
    } else {
        format!("(?i){}", query)
    };
    regex::Regex::new(&pat).map_err(|e| e.to_string())
}

fn regex_find_all(text: &str, query: &str, case_sensitive: bool) -> Result<Vec<(usize, usize)>, String> {
    let re = build_regex(query, case_sensitive)?;
    let mut out = Vec::new();
    for m in re.find_iter(text) {
        let char_start = text[..m.start()].chars().count();
        let char_len = m.as_str().chars().count();
        out.push((char_start, char_len));
    }
    Ok(out)
}