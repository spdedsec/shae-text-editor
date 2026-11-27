//! Syntax highlighting via syntect, with a fast per-line parse cache.

use crate::theme::Palette;
use egui::Color32;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

/// Map a syntect style to an editor color given the active palette.
fn style_to_color(style: Style, p: &Palette) -> Color32 {
    let s = style.font_style;
    // syntect default theme isn't used; we map by font style + a hash of the color
    // to our palette. We approximate by using the foreground color directly when it
    // differs from default, otherwise fall back to palette fg.
    let c = style.foreground;
    // If the highlighter returned a near-default color, use palette fg.
    let fg = Color32::from_rgb(c.r, c.g, c.b);
    // Heuristic: comments are italic in most themes.
    if s.contains(FontStyle::ITALIC) {
        return p.syn_comment;
    }
    // Use the color as-is but nudge very dark colors toward fg_dim for legibility.
    let brightness = (c.r as u16 + c.g as u16 + c.b as u16) / 3;
    if brightness < 40 {
        return p.fg;
    }
    let _ = fg;
    Color32::from_rgb(c.r, c.g, c.b)
}

/// Highlighted spans for a single line: (char_start, char_end, Color32).
pub type LineSpans = Vec<(usize, usize, Color32)>;

pub struct Highlighter {
    pub syntax_set: SyntaxSet,
    pub theme: Theme,
}

impl Highlighter {
    pub fn load_defaults() -> Self {
        let theme_set = syntect::highlighting::ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Highlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }

    pub fn syntax_for_path(&self, path: Option<&Path>) -> Option<&SyntaxReference> {
        let path = path?;
        self.syntax_set
            .find_syntax_for_file(path)
            .ok()
            .flatten()
            .or_else(|| self.syntax_set.find_syntax_by_first_line(""))
    }

    /// Highlight a single line, continuing parse state from previous lines.
    pub fn highlight_line(
        &self,
        state: &mut HighlightLines<'_>,
        line: &str,
        palette: &Palette,
    ) -> LineSpans {
        let mut spans = Vec::new();
        let mut char_pos = 0usize;
        for range in LinesWithEndings::from(line) {
            let line_with_nl = range;
            let regions = state.highlight_line(line_with_nl, &self.syntax_set);
            match regions {
                Ok(regions) => {
                    for (style, text) in regions {
                        let start = char_pos;
                        let len = text.chars().count();
                        let end = start + len;
                        let color = style_to_color(style, palette);
                        spans.push((start, end, color));
                        char_pos = end;
                    }
                }
                Err(_) => {
                    spans.push((char_pos, char_pos + line.chars().count(), palette.fg));
                }
            }
            break; // single line
        }
        spans
    }
}

/// Detect a human-friendly language name from a path.
pub fn language_name(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return "Plain Text".to_string();
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let name = match ext {
        "rs" => "Rust",
        "py" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" => "TypeScript",
        "tsx" => "TypeScript React",
        "jsx" => "JavaScript React",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "sh" | "bash" | "zsh" => "Shell",
        "toml" => "TOML",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "md" | "markdown" => "Markdown",
        "html" | "htm" => "HTML",
        "css" | "scss" | "sass" => "CSS",
        "lua" => "Lua",
        "php" => "PHP",
        "sql" => "SQL",
        "xml" => "XML",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        _ => "Plain Text",
    };
    name.to_string()
}