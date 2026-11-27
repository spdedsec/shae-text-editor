# Shae Text Editor

A fast, smooth, GPU-accelerated native text editor written in Rust with [egui](https://github.com/emilk/egui). Built for speed — sub-frame input latency, GPU-rendered text, and a rope-based buffer that stays responsive on large files.

```
  ◈ SHAE  Text Editor
```

> **New here?** Read the complete [Download & User Guide](GUIDE.md) — install, first launch, every feature, and the full shortcut reference.
>
> **Install in one line (Linux x86_64):**
> ```bash
> curl -sL https://github.com/spdedsec/shae-text-editor/releases/latest/download/shae-text-editor-linux-x86_64.tar.gz | tar xz
> cd shae-text-editor && ./install.sh
> ```

## Install

| Platform | Method |
| --- | --- |
| Linux x86_64 | [Download the release](https://github.com/spdedsec/shae-text-editor/releases/latest), extract, run `./install.sh` |
| Any platform | `cargo install --git https://github.com/spdedsec/shae-text-editor` |
| From source | `git clone … && cargo build --release` |

Full step-by-step instructions, requirements, and troubleshooting live in **[GUIDE.md](GUIDE.md)**.

## Why

Most "fast" editors are either web apps in a browser wrapper or heavyweight IDEs. Shae is a single native binary: no Electron, no runtime, no install ceremony. The entire UI is rendered on the GPU through egui's glow backend, text is stored in a [rope](https://github.com/cessen/ropey), and syntax highlighting is incremental via [syntect](https://github.com/trishume/syntect).

## Features

**Editing**
- Multi-cursor (Ctrl+click to add, click+drag to select)
- Find & replace with case sensitivity, whole-word, and regex modes
- Auto-indent, bracket-aware newline insertion
- Move / duplicate lines (Alt+↑ / Alt+↓, Ctrl+D)
- Toggle line comments (Ctrl+/)
- Word / grapheme-aware cursor movement and deletion
- Full undo / redo

**Views**
- Tabs with dirty indicators
- Line numbers, current-line highlight
- Word wrap (Alt+Z)
- Minimap with viewport box
- Five themes (Shae Dark/Light, Nord, Solarized Dark, Dracula) — Ctrl→ cycle from palette

**Files**
- Open / Save / Save As / New / Close (Ctrl+N/O/S/W)
- Auto-save (configurable interval)
- Git gutter showing added/modified lines per file (via libgit2)
- Live file watcher for external changes

**Power**
- Command palette (Ctrl+P) with fuzzy matching (nucleo)
- Configurable font size, tab width, spaces vs tabs
- Persistent config (`~/.config/shae/ShaeTextEditor/config.toml`)

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| New / Open / Save / Save As | Ctrl+N / O / S / Shift+S |
| Close tab | Ctrl+W |
| Next / Prev tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Command palette | Ctrl+P (or Ctrl+K) |
| Find / Replace | Ctrl+F / Ctrl+H |
| Undo / Redo | Ctrl+Z / Ctrl+Y (or Ctrl+Shift+Z) |
| Duplicate line | Ctrl+D |
| Move line up/down | Alt+↑ / Alt+↓ |
| Toggle comment | Ctrl+/ |
| Delete word back/fwd | Ctrl+Backspace / Ctrl+Delete |
| Word wrap | Alt+Z |
| Zoom font | Ctrl+= / Ctrl+- |
| Quit | Ctrl+Q |

## Build

Requires a Rust toolchain (stable) and system OpenGL. On Linux, libgit2 is used if present (otherwise vendored).

```bash
cargo build --release
./target/release/shae-text-editor
```

The optimized binary is a single ~10 MB executable with the JetBrains Mono font embedded.

## Architecture

| Module | Responsibility |
| --- | --- |
| `buffer.rs` | Rope-backed document, dirty tracking, line-ending normalization |
| `editor.rs` | Multi-cursor state, input handling, GPU text rendering, undo/redo |
| `highlight.rs` | Incremental syntect highlighting with a per-document cache |
| `find.rs` | Find/replace engine (literal + regex) |
| `git.rs` | Per-line git gutter via libgit2 |
| `palette.rs` | Fuzzy command palette (nucleo) |
| `theme.rs` | Color palettes |
| `config.rs` | Persistent TOML config |
| `app.rs` | Tabs, panels, menus, status bar, global shortcuts |

## License

MIT — see [LICENSE](LICENSE).