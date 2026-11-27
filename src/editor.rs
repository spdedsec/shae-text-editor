//! The core text editing widget: multi-cursor state, input handling and GPU rendering.

use crate::buffer::Document;
use crate::config::Config;
use crate::highlight::{Highlighter, LineSpans};
use crate::theme::{Palette, ThemeId};
use egui::{Color32, Context, FontId, Pos2, Rect, Sense, Vec2};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Selection anchor (char index).
    pub anchor: usize,
    /// Active head (char index).
    pub head: usize,
}

impl Cursor {
    pub fn new(pos: usize) -> Self {
        Cursor { anchor: pos, head: pos }
    }
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
    pub fn min(&self) -> usize {
        self.anchor.min(self.head)
    }
    pub fn max(&self) -> usize {
        self.anchor.max(self.head)
    }
    pub fn set(&mut self, pos: usize) {
        self.anchor = pos;
        self.head = pos;
    }
}

/// A single undo/redo operation.
#[derive(Debug, Clone)]
pub enum EditOp {
    /// Insert `text` at `pos`. Undo removes it.
    Insert { pos: usize, text: String },
    /// Delete `count` chars at `pos`; `text` is what was removed. Undo restores it.
    Delete { pos: usize, text: String },
}

#[derive(Debug)]
pub struct EditorState {
    pub cursors: Vec<Cursor>,
    /// Pixel scroll offset (x includes gutter, y is vertical).
    pub scroll: Vec2,
    /// Desired horizontal column (in chars) for vertical moves.
    pub desired_col: Option<usize>,
    pub undo: Vec<EditOp>,
    pub redo: Vec<EditOp>,
    /// Cached highlight spans for the whole document.
    spans: Vec<LineSpans>,
    spans_version: u64,
    spans_theme: ThemeId,
    /// I-beam hover position for cursor shape.
    pub pointer_in: bool,
}

impl EditorState {
    pub fn new() -> Self {
        EditorState {
            cursors: vec![Cursor::new(0)],
            scroll: Vec2::ZERO,
            desired_col: None,
            undo: Vec::new(),
            redo: Vec::new(),
            spans: Vec::new(),
            spans_version: u64::MAX,
            spans_theme: ThemeId::ShaeDark,
            pointer_in: false,
        }
    }

    pub fn primary(&self) -> &Cursor {
        &self.cursors[0]
    }

    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[0]
    }

    fn sort_cursors(&mut self) {
        self.cursors.sort_by_key(|c| c.min());
        self.cursors.dedup_by_key(|c| c.min());
    }

    /// Ensure the highlight cache matches the document + theme.
    fn ensure_spans(&mut self, doc: &Document, theme: ThemeId, hl: &Highlighter, palette: &Palette) {
        if self.spans_version == doc.version && self.spans_theme == theme {
            return;
        }
        self.spans.clear();
        self.spans.reserve(doc.len_lines());
        match hl.syntax_for_path(doc.path.as_deref()) {
            Some(syntax) => {
                use syntect::easy::HighlightLines;
                let mut state = HighlightLines::new(syntax, &hl.theme);
                let text = doc.text();
                for line in text.lines() {
                    // Reconstruct with newline to keep parser state consistent.
                    let line_nl = format!("{}\n", line);
                    let spans = hl.highlight_line(&mut state, &line_nl, palette);
                    self.spans.push(spans);
                }
                // Handle trailing line count mismatch (lines() drops final empty).
                while self.spans.len() < doc.len_lines() {
                    self.spans.push(vec![(0, 0, palette.fg)]);
                }
            }
            None => {
                for i in 0..doc.len_lines() {
                    let n = doc.line_char_len(i);
                    self.spans.push(vec![(0, n, palette.fg)]);
                }
            }
        }
        self.spans_version = doc.version;
        self.spans_theme = theme;
    }
}

/// Metrics computed once per frame.
pub struct Metrics {
    pub char_w: f32,
    pub line_h: f32,
    pub font: FontId,
    pub gutter_w: f32,
}

pub fn compute_metrics(ui: &egui::Ui, cfg: &Config) -> Metrics {
    let font = FontId::monospace(cfg.font_size);
    let char_w = ui
        .fonts(|f| f.layout_no_wrap("0123456789".to_string(), font.clone(), Color32::WHITE).rect.width())
        / 10.0;
    let line_h = cfg.font_size * 1.55;
    let gutter_w = if cfg.line_numbers {
        (measure_gutter(cfg) + 8.0).max(28.0)
    } else {
        4.0
    };
    Metrics { char_w, line_h, font, gutter_w }
}

fn measure_gutter(cfg: &Config) -> f32 {
    // Approximate: digits width * max digits. Refined at render with actual line count.
    cfg.font_size * 0.6 * 4.0
}

/// Result of handling input, signaling the app to perform global actions.
#[derive(Default)]
pub struct EditorOutput {
    pub changed: bool,
    pub cursor_moved: bool,
}

pub struct EditorContext<'a> {
    pub ui: &'a mut egui::Ui,
    pub ctx: &'a Context,
    pub cfg: &'a Config,
    pub palette: &'a Palette,
    pub hl: &'a Highlighter,
}

impl EditorState {
    /// Render and handle input for the document in the given rect. Returns output flags.
    pub fn show(&mut self, ec: &mut EditorContext<'_>, doc: &mut Document, rect: Rect) -> EditorOutput {
        let mut out = EditorOutput::default();
        let metrics = compute_metrics(ec.ui, ec.cfg);
        self.ensure_spans(doc, ec.cfg.theme, ec.hl, ec.palette);

        // Wrap text changes line layout; for v1 we only visually wrap without altering indices.
        let wrap = ec.cfg.word_wrap;
        let wrap_width = (rect.width() - metrics.gutter_w - 16.0).max(40.0);

        // Compute visible line range.
        let first_line = (self.scroll.y / metrics.line_h).floor() as usize;
        let visible = ((rect.height() / metrics.line_h).ceil() as usize) + 2;
        let last_line = (first_line + visible).min(doc.len_lines());

        let text_left = rect.left() + metrics.gutter_w;
        let top = rect.top() - self.scroll.y;

        // Background.
        ec.ui.painter().rect_filled(rect, 0.0, ec.palette.bg);

        // Current line highlight.
        if ec.cfg.highlight_current_line {
            let cur_line = doc.pos_of(self.primary().head).0;
            if cur_line < doc.len_lines() {
                let y = top + cur_line as f32 * metrics.line_h;
                let r = Rect::from_min_size(
                    Pos2::new(rect.left(), y),
                    Vec2::new(rect.width(), metrics.line_h),
                );
                if r.intersects(rect) {
                    ec.ui.painter().rect_filled(r, 0.0, ec.palette.bg_highlight);
                }
            }
        }

        // Selection rectangles (drawn behind text).
        self.paint_selections(ec, doc, rect, &metrics, top, first_line, last_line, wrap, wrap_width);

        // Lines + git gutter + minimap handled elsewhere.
        let lh = metrics.line_h;
        for line_idx in first_line..last_line {
            let y = top + line_idx as f32 * lh;
            let line = doc.line(line_idx);
            let raw = line.to_string();
            let content = raw.strip_suffix('\n').unwrap_or(&raw);
            let content_cr = content.strip_suffix('\r').unwrap_or(content);

            // Line number.
            if ec.cfg.line_numbers {
                let num = format!("{:>width$}", line_idx + 1, width = gutter_digits(doc.len_lines()));
                let color = if line_idx == doc.pos_of(self.primary().head).0 {
                    ec.palette.fg
                } else {
                    ec.palette.fg_faint
                };
                ec.ui.painter().text(
                    Pos2::new(rect.left() + metrics.gutter_w - 8.0, y + lh * 0.5),
                    egui::Align2::RIGHT_CENTER,
                    num,
                    metrics.font.clone(),
                    color,
                );
            }

            // Text spans.
            let spans = &self.spans[line_idx];
            self.paint_line(ec, content_cr, spans, text_left - self.scroll.x, y, &metrics, wrap, wrap_width);

            // Trailing newline indicator / whitespace handled minimally.
        }

        // Cursors.
        self.paint_cursors(ec, doc, rect, &metrics, top, first_line, last_line);

        // Minimap.
        if ec.cfg.minimap {
            self.paint_minimap(ec, doc, rect, &metrics);
        }

        // Input.
        let sub = self.handle_input(ec, doc, rect, &metrics, wrap, wrap_width);
        out.changed |= sub.changed;
        out.cursor_moved |= sub.cursor_moved;

        // Clamp scroll.
        self.clamp_scroll(doc, rect, &metrics);
        out
    }

    fn paint_line(
        &self,
        ec: &mut EditorContext<'_>,
        content: &str,
        spans: &LineSpans,
        x: f32,
        y: f32,
        m: &Metrics,
        wrap: bool,
        wrap_width: f32,
    ) {
        let painter = ec.ui.painter();
        if !wrap {
            let mut col = 0usize;
            for (s, e, color) in spans {
                if *s > col {
                    // gap (shouldn't usually happen)
                }
                let text: String = content.chars().skip(*s).take(e.saturating_sub(*s)).collect();
                if !text.is_empty() {
                    let px = x + *s as f32 * m.char_w;
                    painter.text(Pos2::new(px, y + m.line_h * 0.5), egui::Align2::LEFT_CENTER, text, m.font.clone(), *color);
                }
                col = *e;
            }
        } else {
            // Word wrap: greedily break by width.
            let max_cols = (wrap_width / m.char_w).floor() as usize;
            let chars: Vec<char> = content.chars().collect();
            let mut line_start = 0usize;
            let mut row = 0usize;
            while line_start < chars.len() || (chars.is_empty() && row == 0) {
                let end = (line_start + max_cols).min(chars.len());
                let slice: String = chars[line_start..end].iter().collect();
                // color: use span lookup at line_start
                let color = self.color_at(spans, line_start);
                let py = y + row as f32 * m.line_h;
                painter.text(Pos2::new(x, py + m.line_h * 0.5), egui::Align2::LEFT_CENTER, slice, m.font.clone(), color);
                if end >= chars.len() {
                    break;
                }
                line_start = end;
                row += 1;
            }
        }
    }

    fn color_at(&self, spans: &LineSpans, col: usize) -> Color32 {
        for (s, e, c) in spans {
            if col >= *s && col < *e {
                return *c;
            }
        }
        Color32::WHITE
    }

    fn paint_selections(
        &self,
        ec: &mut EditorContext<'_>,
        doc: &Document,
        rect: Rect,
        m: &Metrics,
        top: f32,
        first: usize,
        last: usize,
        _wrap: bool,
        _wrap_width: f32,
    ) {
        let painter = ec.ui.painter();
        for cur in &self.cursors {
            if cur.is_empty() {
                continue;
            }
            let (sl, sc) = doc.pos_of(cur.min());
            let (el, ec_col) = doc.pos_of(cur.max());
            if el < first || sl > last {
                continue;
            }
            for line in sl..=el {
                if line >= doc.len_lines() {
                    break;
                }
                let start_col = if line == sl { sc } else { 0 };
                let end_col = if line == el {
                    ec_col
                } else {
                    doc.line_char_len(line)
                };
                let y = top + line as f32 * m.line_h;
                let x0 = rect.left() + m.gutter_w + start_col as f32 * m.char_w - self.scroll.x;
                let x1 = rect.left() + m.gutter_w + end_col as f32 * m.char_w - self.scroll.x;
                let r = Rect::from_min_size(Pos2::new(x0, y), Vec2::new((x1 - x0).max(0.0), m.line_h));
                painter.rect_filled(r, 0.0, ec.palette.bg_selection);
            }
        }
    }

    fn paint_cursors(
        &self,
        ec: &mut EditorContext<'_>,
        doc: &Document,
        rect: Rect,
        m: &Metrics,
        top: f32,
        first: usize,
        last: usize,
    ) {
        let painter = ec.ui.painter();
        for cur in &self.cursors {
            let (l, c) = doc.pos_of(cur.head);
            if l < first || l > last {
                continue;
            }
            let y = top + l as f32 * m.line_h;
            let x = rect.left() + m.gutter_w + c as f32 * m.char_w - self.scroll.x;
            let h = m.line_h * 0.92;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x, y + (m.line_h - h) * 0.5), Vec2::new(1.6, h)),
                0.0,
                ec.palette.cursor,
            );
        }
    }

    fn paint_minimap(&self, ec: &mut EditorContext<'_>, doc: &Document, rect: Rect, _m: &Metrics) {
        let mm_w = 90.0;
        let mm_x = rect.right() - mm_w - 6.0;
        let mm_rect = Rect::from_min_size(Pos2::new(mm_x, rect.top() + 4.0), Vec2::new(mm_w, rect.height() - 8.0));
        ec.ui.painter().rect_filled(mm_rect, 0.0, ec.palette.bg_alt);
        let lines = doc.len_lines();
        if lines == 0 {
            return;
        }
        let row_h = (mm_rect.height() / lines as f32).min(2.0);
        let painter = ec.ui.painter();
        for i in 0..lines {
            let len = doc.line_char_len(i);
            let w = ((len as f32) * 0.9).min(mm_w - 8.0);
            let y = mm_rect.top() + i as f32 * row_h;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(mm_x + 4.0, y), Vec2::new(w, row_h.max(0.6))),
                0.0,
                ec.palette.fg_faint.gamma_multiply(0.7),
            );
        }
        // Viewport box.
        let total_h = lines as f32 * row_h;
        let view_frac = (rect.height() / (doc.len_lines() as f32 * _m.line_h)).clamp(0.05, 1.0);
        let box_h = (total_h * view_frac).max(6.0);
        let box_y = mm_rect.top() + (self.scroll.y / _m.line_h) * row_h;
        painter.rect_stroke(
            Rect::from_min_size(Pos2::new(mm_x + 1.0, box_y), Vec2::new(mm_w - 2.0, box_h)),
            0.0,
            egui::Stroke::new(1.0f32, ec.palette.accent),
            egui::StrokeKind::Inside,
        );
    }

    fn clamp_scroll(&mut self, doc: &Document, rect: Rect, m: &Metrics) {
        let max_y = (doc.len_lines() as f32 * m.line_h - rect.height()).max(0.0);
        self.scroll.y = self.scroll.y.clamp(0.0, max_y);
        // Horizontal: estimate longest visible line.
        let mut max_x = 0.0f32;
        let first = (self.scroll.y / m.line_h).floor() as usize;
        let vis = (rect.height() / m.line_h).ceil() as usize + 2;
        for i in first..(first + vis).min(doc.len_lines()) {
            max_x = max_x.max(doc.line_char_len(i) as f32 * m.char_w);
        }
        let max_scroll_x = (max_x - (rect.width() - m.gutter_w - 100.0)).max(0.0);
        self.scroll.x = self.scroll.x.clamp(0.0, max_scroll_x);
    }

    // ----- Input handling -----

    fn handle_input(
        &mut self,
        ec: &mut EditorContext<'_>,
        doc: &mut Document,
        rect: Rect,
        m: &Metrics,
        wrap: bool,
        wrap_width: f32,
    ) -> EditorOutput {
        let mut out = EditorOutput::default();
        let ctx = ec.ctx;
        let cfg = ec.cfg;

        // Allocate interaction over the text area (exclude minimap region).
        let interact_rect = if cfg.minimap {
            Rect::from_min_max(rect.min, Pos2::new(rect.right() - 100.0, rect.max.y))
        } else {
            rect
        };
        let response = ec.ui.allocate_rect(interact_rect, Sense::click_and_drag());
        self.pointer_in = response.hovered();
        if response.hovered() {
            ec.ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }

        // Mouse wheel scroll.
        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
        if response.hovered() && (scroll_delta.x != 0.0 || scroll_delta.y != 0.0) {
            let mods = ctx.input(|i| i.modifiers);
            if mods.shift {
                self.scroll.x += (scroll_delta.y + scroll_delta.x) * cfg.scroll_speed;
            } else {
                self.scroll.y += scroll_delta.y * cfg.scroll_speed;
                self.scroll.x += scroll_delta.x * cfg.scroll_speed;
            }
        }

        // Pointer interactions for cursor placement.
        if let Some(pointer) = response.interact_pointer_pos() {
            if response.drag_started() || response.clicked() {
                let pos = self.point_to_cursor(pointer, doc, rect, m, self.scroll);
                let multi = if response.clicked() && ctx.input(|i| i.modifiers.ctrl || i.modifiers.command) {
                    // toggle/add cursor
                    self.cursors.push(Cursor::new(pos));
                    self.sort_cursors();
                    false
                } else {
                    true
                };
                if multi {
                    if !self.cursors.is_empty() && response.clicked() {
                        self.cursors.truncate(1);
                    }
                    self.primary_mut().set(pos);
                }
                self.desired_col = None;
            }
            if response.dragged() {
                let pos = self.point_to_cursor(pointer, doc, rect, m, self.scroll);
                self.primary_mut().head = pos;
                self.desired_col = None;
            }
            if response.double_clicked() {
                let pos = self.point_to_cursor(pointer, doc, rect, m, self.scroll);
                let (a, b) = word_bounds(doc, pos);
                self.cursors.truncate(1);
                self.primary_mut().anchor = a;
                self.primary_mut().head = b;
            }
            if response.triple_clicked() {
                let pos = self.point_to_cursor(pointer, doc, rect, m, self.scroll);
                let (l, _) = doc.pos_of(pos);
                let a = doc.char_index(l, 0);
                let b = doc.char_index(l, doc.line_char_len(l));
                self.cursors.truncate(1);
                self.primary_mut().anchor = a;
                self.primary_mut().head = b;
            }
        }

        // Keyboard.
        let events = ctx.input(|i| i.events.clone());
        let mods = ctx.input(|i| i.modifiers);
        for ev in &events {
            match ev {
                egui::Event::Text(text) => {
                    // Ignore control combos (handled as keys).
                    out.changed |= self.insert_text(doc, text, cfg);
                }
                egui::Event::Paste(text) => {
                    out.changed |= self.insert_text(doc, text, cfg);
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if self.handle_key(ec, doc, key, modifiers, m, wrap, wrap_width, &mut out) {
                        break;
                    }
                }
                _ => {}
            }
        }
        let _ = mods;

        // Keep at least one cursor.
        if self.cursors.is_empty() {
            self.cursors.push(Cursor::new(0));
        }
        self.sort_cursors();
        out
    }

    /// Returns true if a global action consumed the key (stop processing).
    fn handle_key(
        &mut self,
        ec: &mut EditorContext<'_>,
        doc: &mut Document,
        key: &egui::Key,
        mods: &egui::Modifiers,
        m: &Metrics,
        wrap: bool,
        wrap_width: f32,
        out: &mut EditorOutput,
    ) -> bool {
        use egui::Key::*;
        let cfg = ec.cfg;
        let ctrl = mods.ctrl || mods.command;
        let shift = mods.shift;

        // Global shortcuts are signaled via output flags the app reads; we set fields.
        if ctrl {
            match key {
                ArrowLeft => { self.move_word(doc, shift, true); out.cursor_moved = true; return false; }
                ArrowRight => { self.move_word(doc, shift, false); out.cursor_moved = true; return false; }
                ArrowUp => { self.move_to_line(doc, shift, -1, m); return false; }
                ArrowDown => { self.move_to_line(doc, shift, 1, m); return false; }
                Home => { self.move_line_start(doc, shift); return false; }
                End => { self.move_line_end(doc, shift); return false; }
                Backspace => { self.delete_word_back(doc); out.changed = true; return false; }
                Delete => { self.delete_word_fwd(doc); out.changed = true; return false; }
                A => { self.select_all(doc); return false; }
                C => { self.copy(ec, doc); return false; }
                X => { self.cut(ec, doc); out.changed = true; return false; }
                Z => {
                    if shift { self.redo(doc); } else { self.undo(doc); }
                    out.changed = true;
                    return false;
                }
                Y => { self.redo(doc); out.changed = true; return false; }
                D => { self.duplicate_line(doc); out.changed = true; return false; }
                Slash => { self.toggle_comment(doc); out.changed = true; return false; }
                Comma => { self.toggle_comment(doc); out.changed = true; return false; }
                _ => {}
            }
            if mods.alt && (key == &ArrowUp || key == &ArrowDown) {
                // handled above by ctrl branch ordering; alt+ctrl not typical
            }
        }

        if mods.alt {
            match key {
                ArrowUp => { self.move_line(doc, -1); out.changed = true; return false; }
                ArrowDown => { self.move_line(doc, 1); out.changed = true; return false; }
                _ => {}
            }
        }

        if !ctrl && !mods.alt {
            match key {
                ArrowLeft => { self.move_horizontal(doc, shift, -1); out.cursor_moved = true; return false; }
                ArrowRight => { self.move_horizontal(doc, shift, 1); out.cursor_moved = true; return false; }
                ArrowUp => { self.move_to_line(doc, shift, -1, m); out.cursor_moved = true; return false; }
                ArrowDown => { self.move_to_line(doc, shift, 1, m); out.cursor_moved = true; return false; }
                Home => { self.move_line_start(doc, shift); return false; }
                End => { self.move_line_end(doc, shift); return false; }
                Enter => { self.insert_newline(doc); out.changed = true; return false; }
                Backspace => { self.backspace(doc); out.changed = true; return false; }
                Delete => { self.delete(doc); out.changed = true; return false; }
                Tab => { self.insert_tab(doc, cfg); out.changed = true; return false; }
                Escape => { let h = self.primary().head; self.cursors.truncate(1); self.primary_mut().set(h); return false; }
                _ => {}
            }
        }
        let _ = (wrap, wrap_width);
        false
    }

    // ----- Editing primitives -----

    fn push_undo(&mut self, op: EditOp) {
        self.undo.push(op);
        if self.undo.len() > 5000 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn insert_text(&mut self, doc: &mut Document, text: &str, cfg: &Config) -> bool {
        if text.is_empty() {
            return false;
        }
        // Replace selections, then insert, from rightmost cursor to leftmost.
        let mut cursors = self.cursors.clone();
        cursors.sort_by_key(|c| c.min());
        let mut changed = false;
        for cur in cursors.iter().rev() {
            let (a, b) = (cur.min(), cur.max());
            if b > a {
                let removed = doc.rope.slice(a..b).to_string();
                doc.delete(a, b - a);
                self.push_undo(EditOp::Delete { pos: a, text: removed });
            }
            let pos = a;
            let to_insert = if text == "\t" && cfg.insert_spaces {
                " ".repeat(cfg.tab_width)
            } else {
                text.to_string()
            };
            doc.insert(pos, &to_insert);
            self.push_undo(EditOp::Insert { pos, text: to_insert.clone() });
            changed = true;
            let new_head = pos + to_insert.chars().count();
            // update this cursor
            if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                c.set(new_head);
            }
        }
        self.desired_col = None;
        changed
    }

    fn insert_newline(&mut self, doc: &mut Document) {
        let cfg_indent = auto_indent(doc, self.primary().head);
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (a, b) = (cur.min(), cur.max());
            if b > a {
                doc.delete(a, b - a);
            }
            let nl = if doc.line_ending == crate::buffer::LineEnding::Crlf { "\r\n" } else { "\n" };
            let insert = format!("{}{}", nl, cfg_indent);
            doc.insert(a, &insert);
            self.push_undo(EditOp::Insert { pos: a, text: insert.clone() });
            let nh = a + insert.chars().count();
            if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                c.set(nh);
            }
        }
        self.desired_col = None;
    }

    fn insert_tab(&mut self, doc: &mut Document, cfg: &Config) {
        // Indent selection or insert tab.
        let has_sel = self.cursors.iter().any(|c| !c.is_empty());
        if has_sel {
            self.indent(doc, true, cfg);
        } else {
            self.insert_text(doc, "\t", cfg);
        }
    }

    fn backspace(&mut self, doc: &mut Document) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (a, b) = (cur.min(), cur.max());
            if b > a {
                let removed = doc.rope.slice(a..b).to_string();
                doc.delete(a, b - a);
                self.push_undo(EditOp::Delete { pos: a, text: removed });
                if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) { c.set(a); }
            } else if a > 0 {
                // Delete previous char (handle CRLF as a unit).
                let prev = doc.char_at(a - 1);
                let count = if prev == Some('\n') && a >= 2 && doc.char_at(a - 2) == Some('\r') { 2 } else { 1 };
                let removed = doc.rope.slice(a - count..a).to_string();
                doc.delete(a - count, count);
                self.push_undo(EditOp::Delete { pos: a - count, text: removed });
                if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) { c.set(a - count); }
            }
        }
        self.desired_col = None;
    }

    fn delete(&mut self, doc: &mut Document) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (a, b) = (cur.min(), cur.max());
            if b > a {
                let removed = doc.rope.slice(a..b).to_string();
                doc.delete(a, b - a);
                self.push_undo(EditOp::Delete { pos: a, text: removed });
            } else if a < doc.len_chars() {
                let next = doc.char_at(a);
                let count = if next == Some('\r') && a + 1 < doc.len_chars() && doc.char_at(a + 1) == Some('\n') { 2 } else { 1 };
                let removed = doc.rope.slice(a..a + count).to_string();
                doc.delete(a, count);
                self.push_undo(EditOp::Delete { pos: a, text: removed });
            }
        }
        self.desired_col = None;
    }

    fn delete_word_back(&mut self, doc: &mut Document) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let a = cur.min();
            if cur.max() > cur.min() { self.backspace(doc); continue; }
            let start = prev_word_start(doc, a);
            let removed = doc.rope.slice(start..a).to_string();
            doc.delete(start, a - start);
            self.push_undo(EditOp::Delete { pos: start, text: removed });
            if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) { c.set(start); }
        }
    }

    fn delete_word_fwd(&mut self, doc: &mut Document) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let a = cur.min();
            if cur.max() > cur.min() { self.delete(doc); continue; }
            let end = next_word_end(doc, a);
            let removed = doc.rope.slice(a..end).to_string();
            doc.delete(a, end - a);
            self.push_undo(EditOp::Delete { pos: a, text: removed });
        }
    }

    // ----- Movement -----

    fn move_horizontal(&mut self, doc: &Document, shift: bool, dir: i32) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let head = cur.head;
            let new = if dir < 0 {
                prev_grapheme(doc, head)
            } else {
                next_grapheme(doc, head)
            };
            if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                if shift { c.head = new; } else { c.set(new); }
            }
        }
        self.desired_col = None;
    }

    fn move_word(&mut self, doc: &Document, shift: bool, backward: bool) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let head = cur.head;
            let new = if backward { prev_word_start(doc, head) } else { next_word_end(doc, head) };
            if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                if shift { c.head = new; } else { c.set(new); }
            }
        }
        self.desired_col = None;
    }

    fn move_to_line(&mut self, doc: &Document, shift: bool, delta: i32, _m: &Metrics) {
        let col = self.desired_col.unwrap_or_else(|| doc.pos_of(self.primary().head).1);
        self.desired_col = Some(col);
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (l, _) = doc.pos_of(cur.head);
            let nl = (l as i64 + delta as i64).clamp(0, doc.len_lines() as i64 - 1) as usize;
            let c = col.min(doc.line_char_len(nl));
            let pos = doc.char_index(nl, c);
            if let Some(cc) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                if shift { cc.head = pos; } else { cc.set(pos); }
            }
        }
    }

    fn move_line_start(&mut self, doc: &Document, shift: bool) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (l, c) = doc.pos_of(cur.head);
            // Toggle: first non-whitespace, else col 0.
            let line = doc.line(l).to_string();
            let first_non_ws = line.chars().position(|ch| ch != ' ' && ch != '\t').unwrap_or(0);
            let target = if c == first_non_ws { 0 } else { first_non_ws };
            let pos = doc.char_index(l, target);
            if let Some(cc) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                if shift { cc.head = pos; } else { cc.set(pos); }
            }
        }
        self.desired_col = None;
    }

    fn move_line_end(&mut self, doc: &Document, shift: bool) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (l, _) = doc.pos_of(cur.head);
            let pos = doc.char_index(l, doc.line_char_len(l));
            if let Some(cc) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) {
                if shift { cc.head = pos; } else { cc.set(pos); }
            }
        }
        self.desired_col = None;
    }

    fn select_all(&mut self, doc: &Document) {
        self.cursors.truncate(1);
        self.primary_mut().anchor = 0;
        self.primary_mut().head = doc.len_chars();
    }

    fn move_line(&mut self, doc: &mut Document, delta: i32) {
        // Move the line(s) at the primary cursor up/down.
        let (l, _) = doc.pos_of(self.primary().head);
        let target = l as i64 + delta as i64;
        if target < 0 || target as usize >= doc.len_lines() {
            return;
        }
        let target = target as usize;
        // Remove line l.
        let start = doc.char_index(l, 0);
        let end = doc.char_index(l, doc.line_char_len(l));
        let remove_end = if end < doc.len_chars() { end + newline_len(doc) } else { end };
        let removed = doc.rope.slice(start..remove_end).to_string();
        doc.delete(start, remove_end - start);
        // Insert at target.
        let insert_pos = doc.char_index(target, 0);
        doc.insert(insert_pos, &removed);
        self.push_undo(EditOp::Delete { pos: start, text: String::new() }); // simplified
        let new_head = insert_pos + doc.line(l.min(doc.len_lines()-1)).len_chars();
        self.primary_mut().set(new_head.min(doc.len_chars()));
    }

    fn duplicate_line(&mut self, doc: &mut Document) {
        let (l, _) = doc.pos_of(self.primary().head);
        let line_text = doc.line(l).to_string();
        let end = doc.char_index(l, doc.line_char_len(l));
        let insert_pos = if end < doc.len_chars() { end + newline_len(doc) } else { end };
        doc.insert(insert_pos, &line_text);
        self.push_undo(EditOp::Insert { pos: insert_pos, text: line_text });
        self.primary_mut().set(insert_pos + doc.line(l).len_chars());
    }

    fn indent(&mut self, doc: &mut Document, add: bool, cfg: &Config) {
        // For each line touched by a selection, add/remove one indent level.
        let cursors = self.cursors.clone();
        let tab = if cfg.insert_spaces { " ".repeat(cfg.tab_width) } else { "\t".to_string() };
        for cur in &cursors {
            let (sl, _) = doc.pos_of(cur.min());
            let (el, _) = doc.pos_of(cur.max());
            for l in sl..=el {
                if add {
                    let pos = doc.char_index(l, 0);
                    doc.insert(pos, &tab);
                } else {
                    let line = doc.line(l).to_string();
                    let remove = line.chars().take(cfg.tab_width).take_while(|c| *c == ' ').count();
                    if remove > 0 {
                        let pos = doc.char_index(l, 0);
                        doc.delete(pos, remove);
                    }
                }
            }
        }
    }

    fn toggle_comment(&mut self, doc: &mut Document) {
        let (sl, _) = doc.pos_of(self.primary().min());
        let (el, _) = doc.pos_of(self.primary().max());
        let marker = "// ";
        for l in sl..=el {
            let line = doc.line(l).to_string();
            let trimmed = line.trim_start();
            let pos = doc.char_index(l, 0);
            if trimmed.starts_with(marker) {
                let indent = line.len() - trimmed.len();
                doc.delete(pos + indent, marker.len());
            } else {
                doc.insert(pos, marker);
            }
        }
        self.push_undo(EditOp::Insert { pos: 0, text: String::new() });
    }

    // ----- Clipboard -----

    fn copy(&self, ec: &mut EditorContext<'_>, doc: &Document) {
        self.copy_impl(ec, doc);
    }

    fn cut(&mut self, ec: &mut EditorContext<'_>, doc: &mut Document) {
        self.copy_impl(ec, doc);
        self.backspace_selection(doc);
    }

    fn copy_impl(&self, ec: &mut EditorContext<'_>, doc: &Document) {
        let mut text = String::new();
        for cur in &self.cursors {
            if cur.max() > cur.min() {
                text.push_str(&doc.rope.slice(cur.min()..cur.max()).to_string());
            }
        }
        if text.is_empty() {
            // Copy current line.
            let (l, _) = doc.pos_of(self.primary().head);
            text = doc.line(l).to_string();
        }
        if !text.is_empty() {
            ec.ctx.copy_text(text);
        }
    }

    fn backspace_selection(&mut self, doc: &mut Document) {
        let cursors = self.cursors.clone();
        for cur in cursors.iter().rev() {
            let (a, b) = (cur.min(), cur.max());
            if b > a {
                let removed = doc.rope.slice(a..b).to_string();
                doc.delete(a, b - a);
                self.push_undo(EditOp::Delete { pos: a, text: removed });
                if let Some(c) = self.cursors.iter_mut().find(|c| c.anchor == cur.anchor && c.head == cur.head) { c.set(a); }
            }
        }
    }

    // ----- Undo / Redo -----

    pub fn undo(&mut self, doc: &mut Document) {
        if let Some(op) = self.undo.pop() {
            match op {
                EditOp::Insert { pos, text } => {
                    let n = text.chars().count();
                    doc.delete(pos, n);
                    self.redo.push(EditOp::Insert { pos, text });
                    self.primary_mut().set(pos);
                }
                EditOp::Delete { pos, text } => {
                    doc.insert(pos, &text);
                    self.redo.push(EditOp::Delete { pos, text: text.clone() });
                    self.primary_mut().set(pos + text.chars().count());
                }
            }
            // Mark not dirty if matches? We conservatively keep dirty.
        }
    }

    pub fn redo(&mut self, doc: &mut Document) {
        if let Some(op) = self.redo.pop() {
            match op {
                EditOp::Insert { pos, text } => {
                    doc.insert(pos, &text);
                    self.undo.push(EditOp::Insert { pos, text: text.clone() });
                    self.primary_mut().set(pos + text.chars().count());
                }
                EditOp::Delete { pos, text } => {
                    let n = text.chars().count();
                    doc.delete(pos, n);
                    self.undo.push(EditOp::Delete { pos, text });
                    self.primary_mut().set(pos);
                }
            }
        }
    }

    // ----- Coordinate helpers -----

    fn point_to_cursor(&self, p: Pos2, doc: &Document, rect: Rect, m: &Metrics, scroll: Vec2) -> usize {
        let x = p.x - rect.left() - m.gutter_w + scroll.x;
        let y = p.y - rect.top() + scroll.y;
        let line = (y / m.line_h).floor().max(0.0) as usize;
        let line = line.min(doc.len_lines().saturating_sub(1));
        let col = (x / m.char_w).round().max(0.0) as usize;
        let col = col.min(doc.line_char_len(line));
        doc.char_index(line, col)
    }
}

// ----- Free helpers -----

fn newline_len(doc: &Document) -> usize {
    if doc.line_ending == crate::buffer::LineEnding::Crlf { 2 } else { 1 }
}

fn auto_indent(doc: &Document, head: usize) -> String {
    let (l, _) = doc.pos_of(head);
    if l == 0 {
        return String::new();
    }
    let prev = doc.line(l.saturating_sub(1)).to_string();
    let indent: String = prev.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let trimmed = prev.trim();
    if trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[') || trimmed.ends_with(':') {
        format!("{}    ", indent)
    } else {
        indent
    }
}

fn prev_grapheme(doc: &Document, pos: usize) -> usize {
    if pos == 0 { return 0; }
    let prefix: String = doc.rope.slice(..pos).chars().collect();
    let g = prefix.graphemes(true).next_back();
    pos - g.map(|s| s.chars().count()).unwrap_or(1)
}

fn next_grapheme(doc: &Document, pos: usize) -> usize {
    let n = doc.len_chars();
    if pos >= n { return n; }
    let suffix: String = doc.rope.slice(pos..).chars().collect();
    let g = suffix.graphemes(true).next();
    pos + g.map(|s| s.chars().count()).unwrap_or(1)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn prev_word_start(doc: &Document, pos: usize) -> usize {
    let mut i = pos;
    while i > 0 {
        let c = doc.char_at(i - 1).unwrap();
        if is_word_char(c) { break; }
        i -= 1;
    }
    while i > 0 && is_word_char(doc.char_at(i - 1).unwrap()) {
        i -= 1;
    }
    i
}

fn next_word_end(doc: &Document, pos: usize) -> usize {
    let n = doc.len_chars();
    let mut i = pos;
    while i < n && !is_word_char(doc.char_at(i).unwrap()) {
        i += 1;
    }
    while i < n && is_word_char(doc.char_at(i).unwrap()) {
        i += 1;
    }
    i
}

fn word_bounds(doc: &Document, pos: usize) -> (usize, usize) {
    if pos >= doc.len_chars() {
        return (pos, pos);
    }
    let c = doc.char_at(pos).unwrap();
    if is_word_char(c) {
        (prev_word_start(doc, pos), next_word_end(doc, pos))
    } else {
        // select run of non-word chars
        let mut a = pos;
        let mut b = pos;
        while a > 0 && !is_word_char(doc.char_at(a - 1).unwrap()) { a -= 1; }
        while b < doc.len_chars() && !is_word_char(doc.char_at(b).unwrap()) { b += 1; }
        (a, b)
    }
}

fn gutter_digits(lines: usize) -> usize {
    lines.to_string().len().max(2)
}