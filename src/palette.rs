//! Command palette with fuzzy matching powered by nucleo.

use nucleo::Matcher;
use nucleo::pattern::{CaseMatching, Normalization, Pattern};

#[derive(Debug, Clone)]
pub struct Command {
    pub id: &'static str,
    pub title: String,
    pub category: String,
    pub shortcut: String,
}

/// A scored match result.
#[derive(Debug, Clone)]
pub struct Match {
    pub command: Command,
    pub score: u32,
}

pub struct Palette {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub commands: Vec<Command>,
    cached: Vec<Match>,
    dirty: bool,
}

impl Palette {
    pub fn new(commands: Vec<Command>) -> Self {
        Palette {
            open: false,
            query: String::new(),
            selected: 0,
            commands,
            cached: Vec::new(),
            dirty: true,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selected = 0;
            self.dirty = true;
        }
    }

    pub fn set_query(&mut self, q: String) {
        self.query = q;
        self.dirty = true;
        self.selected = 0;
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.cached.is_empty() {
            return;
        }
        let n = self.cached.len() as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
    }

    /// Recompute matches if the query changed.
    pub fn results(&mut self) -> &[Match] {
        if self.dirty {
            self.recompute();
            self.dirty = false;
        }
        &self.cached
    }

    pub fn selected_command(&self) -> Option<&Command> {
        self.cached.get(self.selected).map(|m| &m.command)
    }

    fn recompute(&mut self) {
        let mut matcher = Matcher::new(nucleo::Config::DEFAULT);
        let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
        let haystacks: Vec<String> = self
            .commands
            .iter()
            .map(|c| format!("{} {}", c.category, c.title))
            .collect();
        // Empty query: keep declaration order, all matched.
        if self.query.trim().is_empty() {
            self.cached = self
                .commands
                .iter()
                .cloned()
                .map(|command| Match { command, score: 0 })
                .collect();
        } else {
            let mut scored: Vec<Match> = Vec::new();
            for (i, hay) in haystacks.iter().enumerate() {
                let needle = nucleo::Utf32String::from(hay.as_str());
                if let Some(score) = pattern.score(needle.slice(..), &mut matcher) {
                    scored.push(Match {
                        command: self.commands[i].clone(),
                        score,
                    });
                }
            }
            scored.sort_by(|a, b| b.score.cmp(&a.score));
            self.cached = scored;
        }
        if self.selected >= self.cached.len() {
            self.selected = self.cached.len().saturating_sub(1);
        }
    }
}