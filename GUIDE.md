# Shae Text Editor — Download & User Guide

Everything you need to download, install, run, and use Shae Text Editor — from first launch to every shortcut and setting.

---

## Table of contents

1. [What you're downloading](#1-what-youre-downloading)
2. [Requirements](#2-requirements)
3. [Download options](#3-download-options)
4. [Install — step by step](#4-install--step-by-step)
5. [First launch](#5-first-launch)
6. [The interface](#6-the-interface)
7. [Everyday use](#7-everyday-use)
8. [Find & replace](#8-find--replace)
9. [Command palette](#9-command-palette)
10. [Tabs, files, and saving](#10-tabs-files-and-saving)
11. [Git gutter](#11-git-gutter)
12. [Themes and appearance](#12-themes-and-appearance)
13. [Configuration file](#13-configuration-file)
14. [Keyboard shortcut reference](#14-keyboard-shortcut-reference)
15. [Build from source](#15-build-from-source)
16. [Troubleshooting](#16-troubleshooting)
17. [Uninstall](#17-uninstall)

---

## 1. What you're downloading

Shae Text Editor is a **single native binary** — no Electron, no Node.js, no runtime to install. The download is a compressed archive containing:

| File | Purpose |
| --- | --- |
| `shae-text-editor` | The application (≈10 MB, optimized build) |
| `install.sh` | One-command installer (copies binary, adds menu entry) |
| `assets/` | App icon + `.desktop` launcher entry |
| `README.md` | Project overview |
| `GUIDE.md` | This guide |
| `LICENSE` | MIT license |

**Release archive:** `shae-text-editor-linux-x86_64.tar.gz`

---

## 2. Requirements

### Operating system
- **Linux x86_64** (the release binary is built for 64-bit Linux).
- Works on Arch/Omarchy, Debian/Ubuntu, Fedora, and most modern distributions.
- macOS and Windows users: [build from source](#15-build-from-source) (takes ~2 minutes).

### Runtime libraries
The binary links against a small set of standard system libraries. Most desktops already have them; the installer checks for you.

| Library | Package name (common distros) |
| --- | --- |
| libgit2 | Arch: `libgit2` · Debian: `libgit2-1.7` · Fedora: `libgit2` |
| OpenSSL | Arch: `openssl` · Debian: `libssl3` · Fedora: `openssl-libs` |
| OpenGL + X11/Wayland | Already present on any graphical desktop |

### Hardware
- Any GPU with working OpenGL (integrated graphics are plenty).
- ~50 MB disk space.
- A display server (X11 or Wayland).

---

## 3. Download options

### Option A — Download the release (recommended)

1. Go to the **Releases** page: <https://github.com/spdedsec/shae-text-editor/releases/latest>
2. Download `shae-text-editor-linux-x86_64.tar.gz`
3. Continue to [Install — step by step](#4-install--step-by-step).

### Option B — Clone and build

```bash
git clone https://github.com/spdedsec/shae-text-editor.git
cd shae-text-editor
cargo build --release
```

See [Build from source](#15-build-from-source) for prerequisites.

---

## 4. Install — step by step

### 4.1 Extract the archive

```bash
tar -xzf shae-text-editor-linux-x86_64.tar.gz
cd shae-text-editor
```

### 4.2 Run the installer

```bash
chmod +x install.sh
./install.sh
```

The installer will:

1. **Check runtime libraries** — and tell you the exact package command if anything is missing.
2. **Copy the binary** to `~/.local/bin/shae-text-editor`.
3. **Add a desktop entry** so it appears in your app launcher.
4. **Install the icon** at every standard size.
5. **Refresh** the desktop and icon caches.

Expected output:

```
==> Checking runtime libraries
  ✓ All runtime libraries present
  ✓ Installed binary -> /home/you/.local/bin/shae-text-editor
  ✓ Installed desktop entry
  ✓ Installed application icons

  ✓ Shae Text Editor installed.
   Run it with:  shae-text-editor
   Or launch "Shae Text Editor" from your app menu.
```

### 4.3 Make sure `~/.local/bin` is on your PATH

If the installer warns that it isn't, add this line to your shell profile (`~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then reload your shell:

```bash
source ~/.bashrc   # or ~/.zshrc
```

### 4.4 Verify

```bash
shae-text-editor --help 2>/dev/null || shae-text-editor
```

A window titled **Shae Text Editor** should open. That's it — you're installed.

> **Prefer no installer?** You can run the binary directly from the extracted folder:
> `./shae-text-editor`. Nothing else is required.

---

## 5. First launch

When the editor opens you get a single empty **Untitled** tab.

- Start typing immediately — there is no splash screen or setup wizard.
- The status bar (bottom right) shows your cursor line/column, language, indentation mode, line endings, and line count.
- The top bar shows the app name and the command-palette hint.

Type a few lines, then press **Ctrl+S** to save. Because the document is untitled, a save dialog opens — choose a name and location.

---

## 6. The interface

```
┌──────────────────────────────────────────────────────────────┐
│ ◈ SHAE   File …                              ⌘K commands      │  top bar
├──────────────────────────────────────────────────────────────┤
│ ● main.rs   notes.txt   +                                     │  tab strip
├──────────────────────────────────────────────────────────────┤
│   1 │ fn main() {                                   ▏        │
│   2 │     println!("hello");                     ▏  ▏        │  editor
│   3 │ }                                         ▏  ▏        │  + minimap
│   4 │                                           ▏  ▏        │
├──────────────────────────────────────────────────────────────┤
│ Ready            Shae Dark · Ln 2, Col 5 · Rust · …          │  status bar
└──────────────────────────────────────────────────────────────┘
```

- **Top bar** — app identity and palette hint.
- **Tab strip** — open files; `●` marks unsaved changes; `+` opens a new tab.
- **Editor** — line numbers on the left, text in the middle, minimap on the right.
- **Status bar** — live position and document info.

---

## 7. Everyday use

### Typing and moving
| You want to… | Do this |
| --- | --- |
| Move by character | `←` / `→` |
| Move by word | `Ctrl+←` / `Ctrl+→` |
| Move by line | `↑` / `↓` |
| Go to line start / end | `Home` / `End` |
| Go to first non-space | `Home` (again) |
| Select while moving | Hold `Shift` with any arrow |
| Select a word | Double-click |
| Select a line | Triple-click |
| Select everything | `Ctrl+A` |

### Editing
| You want to… | Do this |
| --- | --- |
| New line with auto-indent | `Enter` |
| Indent / outdent selection | `Tab` / `Shift+Tab` |
| Delete previous word | `Ctrl+Backspace` |
| Delete next word | `Ctrl+Delete` |
| Duplicate current line | `Ctrl+D` |
| Move line up / down | `Alt+↑` / `Alt+↓` |
| Toggle `// ` comment | `Ctrl+/` |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` (or `Ctrl+Shift+Z`) |
| Copy / cut / paste | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |

### Multiple cursors
- **Ctrl+click** anywhere to add another cursor.
- Click and drag to select with the primary cursor.
- Press `Escape` to collapse back to one cursor.

---

## 8. Find & replace

Press **Ctrl+F** to open the find bar.

- Type to search — matches are counted and highlighted live (`3/17` means third of seventeen).
- **◂ / ▸** jump to the previous/next match.
- **Aa** toggles case sensitivity.
- **R** toggles regular-expression mode.
- **✕** closes the bar.

Press **Ctrl+H** (or use the second row) to reveal **Replace**:

- **Replace** swaps the current match.
- **Replace All** replaces every match in the document and reports the count.

> Invalid regex patterns show a red error message inline instead of crashing.

---

## 9. Command palette

Press **Ctrl+P** to open the fuzzy command palette.

- Start typing any part of a command name — e.g. `save`, `wrap`, `theme`, `palette`.
- Results are ranked by match quality (powered by nucleo, the same matcher family used by Helix).
- `↑` / `↓` to move, `Enter` to run, `Esc` to close.
- You can also click a result.

Available commands include: New/Open/Save/Save As/Close, Find/Replace, Command Palette, Cycle Theme, Toggle Word Wrap, Toggle Minimap, Toggle Line Numbers, Zoom In/Out, and Quit.

---

## 10. Tabs, files, and saving

| Action | Shortcut |
| --- | --- |
| New file | `Ctrl+N` |
| Open file… | `Ctrl+O` |
| Save | `Ctrl+S` |
| Save As… | `Ctrl+Shift+S` |
| Close tab | `Ctrl+W` |
| Next tab | `Ctrl+Tab` |
| Previous tab | `Ctrl+Shift+Tab` |
| Quit | `Ctrl+Q` |

- The tab title shows `●` when there are unsaved changes.
- **Auto-save** can be enabled in the config (see below); it writes dirty files on a fixed interval.
- **Line endings** are detected per file (LF or CRLF) and preserved on save.
- Shae watches the open file's folder, so external edits are noticed.

---

## 11. Git gutter

When you open a file inside a Git repository, Shae computes per-line change markers against `HEAD` using libgit2:

- **Green** — line added
- **Amber** — line modified
- **Red** — line deleted (shown on the following line)

The gutter refreshes about once per second after you stop typing, so it stays accurate without slowing down input.

> Not in a Git repo? The gutter simply stays empty — nothing to configure.

---

## 12. Themes and appearance

Five themes ship built in:

1. **Shae Dark** — warm near-black with an oxide-red signal (default)
2. **Shae Light** — warm paper, near-black ink
3. **Nord**
4. **Solarized Dark**
5. **Dracula**

Cycle themes from the command palette (**Ctrl+P** → *Cycle Theme*) or by setting `theme` in the config file.

**Zoom** the font with `Ctrl+=` / `Ctrl+-`. Everything else (minimap, line numbers, word wrap) toggles from the palette or config.

---

## 13. Configuration file

Shae saves settings to:

```
~/.config/shae/ShaeTextEditor/config.toml
```

It is created on first save/quit and is plain TOML:

```toml
theme = "ShaeDark"        # ShaeDark | ShaeLight | Nord | SolarizedDark | Dracula
font_size = 14.0
tab_width = 4
insert_spaces = true
word_wrap = false
line_numbers = true
minimap = true
auto_save = false
auto_save_secs = 30
highlight_current_line = true
show_whitespace = false
scroll_speed = 1.0
```

Edit it while Shae is closed, then relaunch to apply. Theme and zoom changes made in-app are written back automatically.

---

## 14. Keyboard shortcut reference

### Files
| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+W` | Close tab |
| `Ctrl+Q` | Quit |

### Editing
| Shortcut | Action |
| --- | --- |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / Cut / Paste |
| `Ctrl+A` | Select all |
| `Ctrl+D` | Duplicate line |
| `Ctrl+/` | Toggle line comment |
| `Alt+↑` / `Alt+↓` | Move line up/down |
| `Ctrl+Backspace` / `Ctrl+Delete` | Delete word back/forward |
| `Tab` / `Shift+Tab` | Indent / outdent |

### Navigation
| Shortcut | Action |
| --- | --- |
| `Ctrl+←` / `Ctrl+→` | Word left/right |
| `Home` / `End` | Line start/end |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next/previous tab |

### View & tools
| Shortcut | Action |
| --- | --- |
| `Ctrl+P` | Command palette |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find & replace |
| `Alt+Z` | Toggle word wrap |
| `Ctrl+=` / `Ctrl+-` | Zoom in/out |
| `Escape` | Collapse cursors / close palette |

---

## 15. Build from source

### Prerequisites
- [Rust](https://rustup.rs) (stable toolchain) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- A C linker (`cc`/`gcc`)
- On Linux: X11/Wayland development headers and OpenGL
- Optional: system `libgit2` (otherwise cargo will vendor it, which needs `cmake`)

### Build

```bash
git clone https://github.com/spdedsec/shae-text-editor.git
cd shae-text-editor
cargo build --release
```

The binary appears at `target/release/shae-text-editor`.

### Run

```bash
./target/release/shae-text-editor
```

### Install system-wide (optional)

```bash
sudo install -m 0755 target/release/shae-text-editor /usr/local/bin/shae-text-editor
sudo install -m 0644 assets/shae-text-editor.desktop /usr/share/applications/
```

### macOS / Windows
The code is cross-platform. Install the Rust toolchain for your OS, then `cargo build --release`. No platform-specific source changes are required.

---

## 16. Troubleshooting

| Symptom | Fix |
| --- | --- |
| `error while loading shared libraries: libgit2.so…` | Install libgit2: Arch `sudo pacman -S libgit2`, Debian `sudo apt install libgit2-1.7`, Fedora `sudo dnf install libgit2`. |
| `shae-text-editor: command not found` | `~/.local/bin` isn't on PATH — see [4.3](#43-make-sure-locallocalbin-is-on-your-path). |
| Window opens but text looks wrong | Ensure a monospace-capable GPU/OpenGL driver is active. The font is embedded, so this is rare. |
| App doesn't start on Wayland | Try `WAYLAND_DISPLAY= shae-text-editor` to force X11, or update your compositor. |
| Build fails on `cmake not found` | Either install cmake, or install system libgit2 so it isn't vendored. |
| Build fails on missing X11 headers | Debian/Ubuntu: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libxkbcommon-dev`. |
| Git gutter stays empty | The file must be inside a Git work tree with at least one commit (untracked repos mark all lines as added). |
| High CPU at idle | This build repaints continuously for lowest input latency; on battery you can lower it by closing the minimap. |

---

## 17. Uninstall

```bash
rm -f  ~/.local/bin/shae-text-editor
rm -f  ~/.local/share/applications/shae-text-editor.desktop
rm -rf ~/.local/share/icons/hicolor/*/apps/shae-text-editor.png
rm -rf ~/.config/shae
```

That removes the binary, menu entry, icons, and config. Your documents are never touched.

---

## License

MIT © 2026 Satya (spdedsec). See [LICENSE](LICENSE).
