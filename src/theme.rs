//! Color themes for Shae Text Editor.
//!
//! Themes are plain data so they can be serialized to config and switched live.

use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThemeId {
    ShaeDark,
    ShaeLight,
    Nord,
    SolarizedDark,
    Dracula,
}

impl ThemeId {
    pub fn all() -> &'static [ThemeId] {
        &[
            ThemeId::ShaeDark,
            ThemeId::ShaeLight,
            ThemeId::Nord,
            ThemeId::SolarizedDark,
            ThemeId::Dracula,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeId::ShaeDark => "Shae Dark",
            ThemeId::ShaeLight => "Shae Light",
            ThemeId::Nord => "Nord",
            ThemeId::SolarizedDark => "Solarized Dark",
            ThemeId::Dracula => "Dracula",
        }
    }

    pub fn palette(&self) -> Palette {
        match self {
            ThemeId::ShaeDark => Palette::shae_dark(),
            ThemeId::ShaeLight => Palette::shae_light(),
            ThemeId::Nord => Palette::nord(),
            ThemeId::SolarizedDark => Palette::solarized_dark(),
            ThemeId::Dracula => Palette::dracula(),
        }
    }
}

/// A full color palette used by the editor surface and syntax highlighting.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub bg_alt: Color32,
    pub bg_gutter: Color32,
    pub bg_highlight: Color32,
    pub bg_selection: Color32,
    pub border: Color32,
    pub fg: Color32,
    pub fg_dim: Color32,
    pub fg_faint: Color32,
    pub accent: Color32,
    pub signal: Color32,
    pub cursor: Color32,
    // syntax
    pub syn_keyword: Color32,
    pub syn_string: Color32,
    pub syn_number: Color32,
    pub syn_comment: Color32,
    pub syn_function: Color32,
    pub syn_type: Color32,
    pub syn_macro: Color32,
    pub syn_attribute: Color32,
    pub syn_operator: Color32,
    // git gutter
    pub git_added: Color32,
    pub git_modified: Color32,
    pub git_deleted: Color32,
}

fn c(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    Color32::from_rgb(r, g, b)
}

impl Palette {
    /// Shae Dark — warm near-black ground, oxide signal (matches the Shae field-manual aesthetic).
    pub fn shae_dark() -> Palette {
        Palette {
            bg: c("#16130F"),
            bg_alt: c("#1E1A15"),
            bg_gutter: c("#12100C"),
            bg_highlight: c("#241E17"),
            bg_selection: c("#3A2A1E"),
            border: c("#2E2820"),
            fg: c("#E8E2D6"),
            fg_dim: c("#A89E8C"),
            fg_faint: c("#6B6354"),
            accent: c("#D98A3D"),
            signal: c("#B3341B"),
            cursor: c("#D98A3D"),
            syn_keyword: c("#E0803A"),
            syn_string: c("#9DB86A"),
            syn_number: c("#C9A66B"),
            syn_comment: c("#6B6354"),
            syn_function: c("#E8C87A"),
            syn_type: c("#7FB3B0"),
            syn_macro: c("#D98A3D"),
            syn_attribute: c("#B0A85C"),
            syn_operator: c("#A89E8C"),
            git_added: c("#7FB36A"),
            git_modified: c("#D9A23D"),
            git_deleted: c("#B3341B"),
        }
    }

    pub fn shae_light() -> Palette {
        Palette {
            bg: c("#F2EEE5"),
            bg_alt: c("#EAE4D8"),
            bg_gutter: c("#ECE6DA"),
            bg_highlight: c("#E4DDCE"),
            bg_selection: c("#E7C9A8"),
            border: c("#D2C9B8"),
            fg: c("#191712"),
            fg_dim: c("#5A5244"),
            fg_faint: c("#8C8372"),
            accent: c("#B3341B"),
            signal: c("#B3341B"),
            cursor: c("#B3341B"),
            syn_keyword: c("#A8431B"),
            syn_string: c("#4F6B2A"),
            syn_number: c("#8A6A1E"),
            syn_comment: c("#8C8372"),
            syn_function: c("#7A5A12"),
            syn_type: c("#1F6A66"),
            syn_macro: c("#A8431B"),
            syn_attribute: c("#6B6A1E"),
            syn_operator: c("#5A5244"),
            git_added: c("#3F7A2A"),
            git_modified: c("#9A6A12"),
            git_deleted: c("#B3341B"),
        }
    }

    pub fn nord() -> Palette {
        Palette {
            bg: c("#2E3440"),
            bg_alt: c("#3B4252"),
            bg_gutter: c("#2A303B"),
            bg_highlight: c("#3B4252"),
            bg_selection: c("#434C5E"),
            border: c("#4C566A"),
            fg: c("#ECEFF4"),
            fg_dim: c("#D8DEE9"),
            fg_faint: c("#7B8394"),
            accent: c("#88C0D0"),
            signal: c("#BF616A"),
            cursor: c("#88C0D0"),
            syn_keyword: c("#81A1C1"),
            syn_string: c("#A3BE8C"),
            syn_number: c("#B48EAD"),
            syn_comment: c("#616E88"),
            syn_function: c("#88C0D0"),
            syn_type: c("#8FBCBB"),
            syn_macro: c("#81A1C1"),
            syn_attribute: c("#EBCB8B"),
            syn_operator: c("#81A1C1"),
            git_added: c("#A3BE8C"),
            git_modified: c("#EBCB8B"),
            git_deleted: c("#BF616A"),
        }
    }

    pub fn solarized_dark() -> Palette {
        Palette {
            bg: c("#002B36"),
            bg_alt: c("#073642"),
            bg_gutter: c("#00252E"),
            bg_highlight: c("#073642"),
            bg_selection: c("#0A4A5A"),
            border: c("#0F4A57"),
            fg: c("#839496"),
            fg_dim: c("#93A1A1"),
            fg_faint: c("#586E75"),
            accent: c("#2AA198"),
            signal: c("#DC322F"),
            cursor: c("#2AA198"),
            syn_keyword: c("#859900"),
            syn_string: c("#2AA198"),
            syn_number: c("#D33682"),
            syn_comment: c("#586E75"),
            syn_function: c("#268BD2"),
            syn_type: c("#B58900"),
            syn_macro: c("#CB4B16"),
            syn_attribute: c("#B58900"),
            syn_operator: c("#859900"),
            git_added: c("#859900"),
            git_modified: c("#B58900"),
            git_deleted: c("#DC322F"),
        }
    }

    pub fn dracula() -> Palette {
        Palette {
            bg: c("#282A36"),
            bg_alt: c("#343746"),
            bg_gutter: c("#21222C"),
            bg_highlight: c("#343746"),
            bg_selection: c("#44475A"),
            border: c("#44475A"),
            fg: c("#F8F8F2"),
            fg_dim: c("#BDC0CC"),
            fg_faint: c("#6272A4"),
            accent: c("#BD93F9"),
            signal: c("#FF5555"),
            cursor: c("#BD93F9"),
            syn_keyword: c("#FF79C6"),
            syn_string: c("#F1FA8C"),
            syn_number: c("#BD93F9"),
            syn_comment: c("#6272A4"),
            syn_function: c("#50FA7B"),
            syn_type: c("#8BE9FD"),
            syn_macro: c("#FF79C6"),
            syn_attribute: c("#50FA7B"),
            syn_operator: c("#FF79C6"),
            git_added: c("#50FA7B"),
            git_modified: c("#F1FA8C"),
            git_deleted: c("#FF5555"),
        }
    }
}