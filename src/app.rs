//! Shae Text Editor — top-level application: tabs, menus, palette, find bar, status bar.

use crate::buffer::Document;
use crate::config::Config;
use crate::editor::{EditorContext, EditorState};
use crate::find::FindState;
use crate::git::GitGutter;
use crate::highlight::{language_name, Highlighter};
use crate::palette::{Command, Palette as CommandPalette};
use crate::theme::ThemeId;
use egui::{Color32, Context, FontId, Pos2, Rect, Vec2};
use std::collections::VecDeque;
use std::path::PathBuf;
use notify::Watcher;
use std::time::Instant;

/// A single open document tab.
pub struct Tab {
    pub doc: Document,
    pub editor: EditorState,
    pub find: FindState,
    pub gutter: GitGutter,
    /// When the gutter was last computed.
    gutter_time: Option<Instant>,
}

impl Tab {
    fn new(doc: Document) -> Self {
        Tab {
            doc,
            editor: EditorState::new(),
            find: FindState::new(),
            gutter: GitGutter::empty(),
            gutter_time: None,
        }
    }
}

/// A pending global action requested by the editor widget.
#[derive(Debug, Clone)]
pub enum GlobalAction {
    Save,
    SaveAs,
    Open,
    New,
    CloseTab,
    NextTab,
    PrevTab,
    TogglePalette,
    ToggleFind,
    ToggleReplace,
    Quit,
    Undo,
    Redo,
}

pub struct ShaeApp {
    pub cfg: Config,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub next_id: u64,
    pub hl: Highlighter,
    pub palette: CommandPalette,
    pub toast: Option<(String, Instant)>,
    pub last_autosave: Instant,
    pub pending_actions: VecDeque<GlobalAction>,
    /// Message log for the bottom status line.
    pub status: String,
    pub quit_requested: bool,
    /// File watcher channel (path, ) for reload notifications — optional.
    watcher_rx: Option<crossbeam_channel::Receiver<notify::Result<notify::Event>>>,
    _watcher: Option<notify::RecommendedWatcher>,
}

impl ShaeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = Config::load();
        let mut hl = Highlighter::load_defaults();
        let _ = &mut hl;
        let commands = build_commands();
        let palette = CommandPalette::new(commands);

        // Open with a single untitled document.
        let doc = Document::new(0, String::new());
        let tabs = vec![Tab::new(doc)];

        ShaeApp {
            cfg,
            tabs,
            active: 0,
            next_id: 1,
            hl,
            palette,
            toast: None,
            last_autosave: Instant::now(),
            pending_actions: VecDeque::new(),
            status: "Ready".to_string(),
            quit_requested: false,
            watcher_rx: None,
            _watcher: None,
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    fn notify(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    // ----- File operations -----

    fn new_file(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        let doc = Document::new(id, String::new());
        self.tabs.push(Tab::new(doc));
        self.active = self.tabs.len() - 1;
        self.status = "New file".into();
    }

    fn open_file(&mut self) {
        let mut picked = None;
        let _ = rfd::FileDialog::new()
            .set_title("Open file")
            .pick_file()
            .map(|p| picked = Some(p));
        if let Some(path) = picked {
            self.load_path(path);
        }
    }

    fn load_path(&mut self, path: PathBuf) {
        let id = self.next_id;
        self.next_id += 1;
        match Document::from_file(id, path.clone()) {
            Ok(doc) => {
                let name = doc.display_name();
                let mut tab = Tab::new(doc);
                tab.gutter = GitGutter::compute(tab.doc.path.as_deref());
                tab.gutter_time = Some(Instant::now());
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
                self.status = format!("Opened {}", name);
                self.setup_watcher();
            }
            Err(e) => {
                self.notify(format!("Failed to open: {}", e));
            }
        }
    }

    fn save_active(&mut self) {
        if self.active_tab().doc.path.is_none() {
            self.save_as();
            return;
        }
        let res = self.active_tab_mut().doc.save();
        match res {
            Ok(()) => {
                let name = self.active_tab().doc.display_name();
                self.status = format!("Saved {}", name);
            }
            Err(e) => self.notify(format!("Save failed: {}", e)),
        }
    }

    fn save_as(&mut self) {
        let default_name = self.active_tab().doc.display_name();
        let picked = rfd::FileDialog::new()
            .set_title("Save file as")
            .set_file_name(&default_name)
            .save_file();
        if let Some(path) = picked {
            self.active_tab_mut().doc.path = Some(path);
            let res = self.active_tab_mut().doc.save();
            match res {
                Ok(()) => {
                    let name = self.active_tab().doc.display_name();
                    self.status = format!("Saved {}", name);
                    self.setup_watcher();
                }
                Err(e) => self.notify(format!("Save failed: {}", e)),
            }
        }
    }

    fn close_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let dirty = self.tabs[self.active].doc.dirty;
        if dirty {
            // For v1, close without prompt but keep content recoverable via undo history loss.
            // A full prompt dialog is a future enhancement.
        }
        self.tabs.remove(self.active);
        if self.tabs.is_empty() {
            let id = self.next_id;
            self.next_id += 1;
            self.tabs.push(Tab::new(Document::new(id, String::new())));
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    // ----- File watcher -----

    fn setup_watcher(&mut self) {
        // Lightweight: create a watcher for the active file's directory.
        let Some(path) = self.active_tab().doc.path.clone() else {
            return;
        };
        let (tx, rx) = crossbeam_channel::unbounded();
        match notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) {
            Ok(mut watcher) => {
                if let Some(parent) = path.parent() {
                    let _ = watcher.watch(parent, notify::RecursiveMode::NonRecursive);
                }
                self.watcher_rx = Some(rx);
                self._watcher = Some(watcher);
            }
            Err(_) => {}
        }
    }
}

fn build_commands() -> Vec<Command> {
    vec![
        cmd("file.new", "New File", "File", "Ctrl+N"),
        cmd("file.open", "Open File…", "File", "Ctrl+O"),
        cmd("file.save", "Save", "File", "Ctrl+S"),
        cmd("file.save_as", "Save As…", "File", "Ctrl+Shift+S"),
        cmd("file.close", "Close Tab", "File", "Ctrl+W"),
        cmd("edit.undo", "Undo", "Edit", "Ctrl+Z"),
        cmd("edit.redo", "Redo", "Edit", "Ctrl+Y"),
        cmd("edit.find", "Find", "Edit", "Ctrl+F"),
        cmd("edit.replace", "Replace", "Edit", "Ctrl+H"),
        cmd("view.palette", "Command Palette", "View", "Ctrl+P"),
        cmd("view.theme", "Cycle Theme", "View", ""),
        cmd("view.toggle_wrap", "Toggle Word Wrap", "View", "Alt+Z"),
        cmd("view.toggle_minimap", "Toggle Minimap", "View", ""),
        cmd("view.toggle_numbers", "Toggle Line Numbers", "View", ""),
        cmd("view.zoom_in", "Increase Font Size", "View", "Ctrl+="),
        cmd("view.zoom_out", "Decrease Font Size", "View", "Ctrl+-"),
        cmd("app.quit", "Quit", "App", "Ctrl+Q"),
    ]
}

fn cmd(id: &'static str, title: &str, category: &str, shortcut: &str) -> Command {
    Command {
        id,
        title: title.to_string(),
        category: category.to_string(),
        shortcut: shortcut.to_string(),
    }
}

impl eframe::App for ShaeApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_global_shortcuts(ctx);
        self.process_pending_actions();
        self.autosave_tick();

        // Top menu/panel bar.
        self.top_bar(ctx);
        // Tab strip.
        self.tab_bar(ctx);
        // Find bar.
        if self.active_tab().find.open {
            self.find_bar(ctx);
        }
        // Editor area.
        self.editor_area(ctx);
        // Status bar.
        self.status_bar(ctx);
        // Command palette overlay.
        if self.palette.open {
            self.palette_overlay(ctx);
        }
        // Toast.
        self.draw_toast(ctx);

        if self.quit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl ShaeApp {
    fn handle_global_shortcuts(&mut self, ctx: &Context) {
        let events = ctx.input(|i| i.events.clone());
        let mods = ctx.input(|i| i.modifiers);
        let ctrl = mods.ctrl || mods.command;
        for ev in &events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = ev {
                if !ctrl {
                    // Alt+Z word wrap.
                    if modifiers.alt && key == &egui::Key::Z {
                        self.cfg.word_wrap = !self.cfg.word_wrap;
                    }
                    continue;
                }
                match key {
                    egui::Key::N => self.pending_actions.push_back(GlobalAction::New),
                    egui::Key::O => self.pending_actions.push_back(GlobalAction::Open),
                    egui::Key::S => {
                        if modifiers.shift {
                            self.pending_actions.push_back(GlobalAction::SaveAs);
                        } else {
                            self.pending_actions.push_back(GlobalAction::Save);
                        }
                    }
                    egui::Key::W => self.pending_actions.push_back(GlobalAction::CloseTab),
                    egui::Key::P => self.pending_actions.push_back(GlobalAction::TogglePalette),
                    egui::Key::F => {
                        if modifiers.shift {
                            self.pending_actions.push_back(GlobalAction::ToggleReplace);
                        } else {
                            self.pending_actions.push_back(GlobalAction::ToggleFind);
                        }
                    }
                    egui::Key::H => self.pending_actions.push_back(GlobalAction::ToggleReplace),
                    egui::Key::Q => self.pending_actions.push_back(GlobalAction::Quit),
                    egui::Key::Tab => {
                        if modifiers.shift {
                            self.pending_actions.push_back(GlobalAction::PrevTab);
                        } else {
                            self.pending_actions.push_back(GlobalAction::NextTab);
                        }
                    }
                    egui::Key::Equals => self.cfg.font_size = (self.cfg.font_size + 1.0).min(48.0),
                    egui::Key::Minus => self.cfg.font_size = (self.cfg.font_size - 1.0).max(6.0),
                    _ => {}
                }
            }
        }
        let _ = mods;
    }

    fn process_pending_actions(&mut self) {
        while let Some(action) = self.pending_actions.pop_front() {
            match action {
                GlobalAction::New => self.new_file(),
                GlobalAction::Open => self.open_file(),
                GlobalAction::Save => self.save_active(),
                GlobalAction::SaveAs => self.save_as(),
                GlobalAction::CloseTab => self.close_tab(),
                GlobalAction::NextTab => {
                    if !self.tabs.is_empty() {
                        self.active = (self.active + 1) % self.tabs.len();
                    }
                }
                GlobalAction::PrevTab => {
                    if !self.tabs.is_empty() {
                        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
                    }
                }
                GlobalAction::TogglePalette => self.palette.toggle(),
                GlobalAction::ToggleFind => self.active_tab_mut().find.toggle(),
                GlobalAction::ToggleReplace => {
                    self.active_tab_mut().find.open = true;
                }
                GlobalAction::Quit => {
                    let _ = self.cfg.save();
                    self.quit_requested = true;
                }
                GlobalAction::Undo | GlobalAction::Redo => {}
            }
        }
    }

    fn autosave_tick(&mut self) {
        if !self.cfg.auto_save {
            return;
        }
        if self.last_autosave.elapsed().as_secs() < self.cfg.auto_save_secs {
            return;
        }
        self.last_autosave = Instant::now();
        for tab in &mut self.tabs {
            if tab.doc.dirty {
                if let Some(path) = tab.doc.path.clone() {
                    let text = tab.doc.text();
                    let _ = std::fs::write(&path, text.as_bytes());
                    tab.doc.dirty = false;
                }
            }
        }
    }

    fn top_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").exact_height(34.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("◈ SHAE")
                        .font(FontId::monospace(15.0))
                        .strong()
                        .color(self.palette_accent()),
                );
                ui.add_space(16.0);
                menu_button(ui, "File", |app| {
                    app.pending_actions.push_back(GlobalAction::New);
                });
                ui.separator();
                ui.label(
                    egui::RichText::new("Text Editor")
                        .font(FontId::monospace(11.0))
                        .color(self.palette_dim()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("⌘K  commands")
                            .font(FontId::monospace(10.0))
                            .color(self.palette_faint()),
                    );
                });
            });
        });
    }

    fn tab_bar(&mut self, ctx: &Context) {
        let n = self.tabs.len();
        egui::TopBottomPanel::top("tab_bar").exact_height(30.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for i in 0..n {
                    let dirty = self.tabs[i].doc.dirty;
                    let active = i == self.active;
                    let name = self.tabs[i].doc.display_name();
                    let label = format!("{}{}", if dirty { "● " } else { "" }, name);
                    let color = if active { self.palette_fg() } else { self.palette_dim() };
                    let btn = egui::Button::new(
                        egui::RichText::new(label).font(FontId::monospace(12.0)).color(color),
                    )
                    .fill(if active { self.palette_bg_alt() } else { self.palette_bg() });
                    if ui.add(btn).clicked() {
                        self.active = i;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("+").font(FontId::monospace(16.0)).color(self.palette_dim()),
                        ))
                        .clicked()
                    {
                        self.new_file();
                    }
                });
            });
        });
    }

    fn find_bar(&mut self, ctx: &Context) {
        let palette_bg = self.palette_bg_alt();
        let palette_fg = self.palette_fg();
        egui::TopBottomPanel::top("find_bar").exact_height(64.0).show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.active_tab_mut().find.query)
                            .desired_width(320.0)
                            .hint_text("Find…"),
                    );
                    if resp.changed() {
                        let tab = self.active_tab_mut();
                        tab.find.recompute(&tab.doc);
                    }
                    // Navigation.
                    let count = self.active_tab().find.matches.len();
                    if ui.button("◂").clicked() {
                        let tab = self.active_tab_mut();
                        tab.find.prev(count);
                        tab.find.recompute(&tab.doc);
                        self.goto_match();
                    }
                    ui.label(format!("{}/{}", if count == 0 { 0 } else { self.active_tab().find.current + 1 }, count));
                    if ui.button("▸").clicked() {
                        let tab = self.active_tab_mut();
                        tab.find.next(count);
                        tab.find.recompute(&tab.doc);
                        self.goto_match();
                    }
                    // Toggles.
                    let cs = self.active_tab().find.case_sensitive;
                    if ui.selectable_label(cs, "Aa").clicked() {
                        self.active_tab_mut().find.case_sensitive = !cs;
                        let tab = self.active_tab_mut();
                        tab.find.recompute(&tab.doc);
                    }
                    let ww = self.active_tab().find.whole_word;
                    if ui.selectable_label(ww, ".*").clicked() {
                        // placeholder
                    }
                    let re = self.active_tab().find.use_regex;
                    if ui.selectable_label(re, "R").clicked() {
                        self.active_tab_mut().find.use_regex = !re;
                        let tab = self.active_tab_mut();
                        tab.find.recompute(&tab.doc);
                    }
                    if let Some(err) = self.active_tab().find.error.clone() {
                        ui.label(egui::RichText::new(err).color(Color32::from_rgb(220, 80, 60)));
                    }
                    if ui.button("✕").clicked() {
                        self.active_tab_mut().find.open = false;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label("→");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.active_tab_mut().find.replacement)
                            .desired_width(240.0)
                            .hint_text("Replace…"),
                    );
                    if ui.button("Replace").clicked() {
                        let tab = self.active_tab_mut();
                        tab.find.replace_current(&mut tab.doc);
                        tab.find.recompute(&tab.doc);
                        self.goto_match();
                    }
                    if ui.button("Replace All").clicked() {
                        let tab = self.active_tab_mut();
                        let n = tab.find.replace_all(&mut tab.doc);
                        self.notify(format!("Replaced {}", n));
                    }
                    ui.label(
                        egui::RichText::new("  ↵ find  ·  Ctrl+H replace")
                            .font(FontId::monospace(10.0))
                            .color(self.palette_faint()),
                    );
                    let _ = (palette_bg, palette_fg);
                });
            });
        });
    }

    fn goto_match(&mut self) {
        let Some((start, _len)) = self.active_tab().find.current_match() else {
            return;
        };
        let line_h = self.cfg.font_size * 1.55;
        let tab = self.active_tab_mut();
        tab.editor.cursors.truncate(1);
        tab.editor.primary_mut().set(start);
        // Scroll into view (rough).
        let m_line = tab.doc.pos_of(start).0;
        tab.editor.scroll.y = (m_line as f32 * line_h - 100.0).max(0.0);
    }

    fn editor_area(&mut self, ctx: &Context) {
        let palette = self.cfg.theme.palette();
        let tab_id = self.active;
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let tab = &mut self.tabs[tab_id];
            let mut ec = EditorContext {
                ui,
                ctx,
                cfg: &self.cfg,
                palette: &palette,
                hl: &self.hl,
            };
            let out = tab.editor.show(&mut ec, &mut tab.doc, rect);
            if out.changed {
                tab.gutter_time = None;
            }
        });
        // Recompute gutter lazily after edit.
        let recompute = self.tabs[tab_id].gutter_time.map(|t| t.elapsed().as_secs() > 1).unwrap_or(true);
        if recompute {
            let path = self.tabs[tab_id].doc.path.clone();
            self.tabs[tab_id].gutter = GitGutter::compute(path.as_deref());
            self.tabs[tab_id].gutter_time = Some(Instant::now());
        }
    }

    fn status_bar(&mut self, ctx: &Context) {
        let (line, col) = {
            let tab = self.active_tab();
            tab.doc.pos_of(tab.editor.primary().head)
        };
        let lang = language_name(self.active_tab().doc.path.as_deref());
        let lines = self.active_tab().doc.len_lines();
        let spaces = if self.cfg.insert_spaces { "Spaces" } else { "Tabs" };
        let enc = match self.active_tab().doc.line_ending {
            crate::buffer::LineEnding::Lf => "LF",
            crate::buffer::LineEnding::Crlf => "CRLF",
        };
        egui::TopBottomPanel::bottom("status_bar").exact_height(24.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(self.status.clone())
                        .font(FontId::monospace(11.0))
                        .color(self.palette_dim()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}  ·  Ln {}, Col {}  ·  {}  ·  {}:{}  ·  {}  ·  {}",
                            self.cfg.theme.name(),
                            line + 1,
                            col + 1,
                            lang,
                            spaces,
                            self.cfg.tab_width,
                            enc,
                            format!("{} lines", lines),
                        ))
                        .font(FontId::monospace(11.0))
                        .color(self.palette_dim()),
                    );
                });
            });
        });
    }

    fn palette_overlay(&mut self, ctx: &Context) {
        // Capture keyboard for the palette query.
        let events = ctx.input(|i| i.events.clone());
        for ev in &events {
            match ev {
                egui::Event::Text(t) => {
                    let q = format!("{}{}", self.palette.query, t);
                    self.palette.set_query(q);
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    match key {
                        egui::Key::Escape => self.palette.open = false,
                        egui::Key::ArrowDown => self.palette.move_selection(1),
                        egui::Key::ArrowUp => self.palette.move_selection(-1),
                        egui::Key::Enter => {
                            if let Some(cmd) = self.palette.selected_command().cloned() {
                                self.palette.open = false;
                                self.execute_command(cmd.id);
                            }
                        }
                        egui::Key::Backspace => {
                            let mut q = self.palette.query.clone();
                            q.pop();
                            self.palette.set_query(q);
                        }
                        _ => {
                            let _ = modifiers;
                        }
                    }
                }
                _ => {}
            }
        }

        let screen = ctx.screen_rect();
        let w = 640.0f32.min(screen.width() - 80.0);
        let h = 420.0f32.min(screen.height() - 120.0);
        let x = (screen.width() - w) / 2.0;
        let y = screen.height() * 0.12;
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
        let response = egui::Area::new(egui::Id::new("palette"))
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(self.palette_bg_alt())
                    .stroke(egui::Stroke::new(1.0f32, self.palette_border()))
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.set_width(w - 16.0);
                        ui.set_height(h - 16.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.palette.query)
                                .desired_width(f32::INFINITY)
                                .hint_text("Type a command…")
                                .font(FontId::monospace(14.0)),
                        );
                        ui.add_space(6.0);
                        ui.separator();
                        let results = self.palette.results().to_vec();
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            for (i, m) in results.iter().enumerate() {
                                let sel = i == self.palette.selected;
                                let text = format!("{:<14}  {}", m.command.category, m.command.title);
                                let color = if sel { self.palette_bg() } else { self.palette_fg() };
                                let fill = if sel { self.palette_accent() } else { self.palette_bg_alt() };
                                let btn = egui::Button::new(
                                    egui::RichText::new(text).font(FontId::monospace(12.0)).color(color),
                                )
                                .fill(fill)
                                .min_size(egui::vec2(0.0, 24.0));
                                let r = ui.add(btn);
                                if r.clicked() {
                                    let id = m.command.id;
                                    self.palette.open = false;
                                    self.execute_command(id);
                                }
                                if r.hovered() {
                                    self.palette.selected = i;
                                }
                            }
                            if results.is_empty() {
                                ui.label(
                                    egui::RichText::new("No matching commands")
                                        .font(FontId::monospace(12.0))
                                        .color(self.palette_faint()),
                                );
                            }
                        });
                    });
            });
        let _ = response;
    }

    fn execute_command(&mut self, id: &str) {
        match id {
            "file.new" => self.new_file(),
            "file.open" => self.open_file(),
            "file.save" => self.save_active(),
            "file.save_as" => self.save_as(),
            "file.close" => self.close_tab(),
            "edit.find" => self.active_tab_mut().find.toggle(),
            "edit.replace" => self.active_tab_mut().find.open = true,
            "view.toggle_wrap" => self.cfg.word_wrap = !self.cfg.word_wrap,
            "view.toggle_minimap" => self.cfg.minimap = !self.cfg.minimap,
            "view.toggle_numbers" => self.cfg.line_numbers = !self.cfg.line_numbers,
            "view.zoom_in" => self.cfg.font_size = (self.cfg.font_size + 1.0).min(48.0),
            "view.zoom_out" => self.cfg.font_size = (self.cfg.font_size - 1.0).max(6.0),
            "view.theme" => self.cycle_theme(),
            "app.quit" => self.quit_requested = true,
            _ => {}
        }
        let _ = Config::save(&self.cfg);
    }

    fn cycle_theme(&mut self) {
        let all = ThemeId::all();
        let idx = all.iter().position(|t| *t == self.cfg.theme).unwrap_or(0);
        self.cfg.theme = all[(idx + 1) % all.len()];
        self.notify(format!("Theme: {}", self.cfg.theme.name()));
    }

    fn draw_toast(&mut self, ctx: &Context) {
        if let Some((msg, t)) = &self.toast {
            if t.elapsed().as_secs() < 3 {
                let screen = ctx.screen_rect();
                let w = 360.0;
                let x = screen.width() - w - 16.0;
                let y = screen.height() - 64.0;
                egui::Area::new(egui::Id::new("toast"))
                    .fixed_pos(Pos2::new(x, y))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::NONE
                            .fill(self.palette_bg_alt())
                            .stroke(egui::Stroke::new(1.0f32, self.palette_accent()))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(msg.clone())
                                        .font(FontId::monospace(12.0))
                                        .color(self.palette_fg()),
                                );
                            });
                    });
            } else {
                self.toast = None;
            }
        }
    }

    // Palette color helpers.
    fn palette_bg(&self) -> Color32 { self.cfg.theme.palette().bg }
    fn palette_bg_alt(&self) -> Color32 { self.cfg.theme.palette().bg_alt }
    fn palette_fg(&self) -> Color32 { self.cfg.theme.palette().fg }
    fn palette_dim(&self) -> Color32 { self.cfg.theme.palette().fg_dim }
    fn palette_faint(&self) -> Color32 { self.cfg.theme.palette().fg_faint }
    fn palette_accent(&self) -> Color32 { self.cfg.theme.palette().accent }
    fn palette_border(&self) -> Color32 { self.cfg.theme.palette().border }
}

fn menu_button(ui: &mut egui::Ui, label: &str, _f: impl FnOnce(&mut ShaeApp)) {
    ui.label(egui::RichText::new(label).font(FontId::monospace(12.0)).color(Color32::from_rgb(180, 170, 150)));
}