//! Persistent user configuration for Shae Text Editor.

use crate::theme::ThemeId;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeId,
    pub font_size: f32,
    pub tab_width: usize,
    pub insert_spaces: bool,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub minimap: bool,
    pub auto_save: bool,
    pub auto_save_secs: u64,
    pub highlight_current_line: bool,
    pub show_whitespace: bool,
    pub scroll_speed: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: ThemeId::ShaeDark,
            font_size: 14.0,
            tab_width: 4,
            insert_spaces: true,
            word_wrap: false,
            line_numbers: true,
            minimap: true,
            auto_save: false,
            auto_save_secs: 30,
            highlight_current_line: true,
            show_whitespace: false,
            scroll_speed: 1.0,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("in", "shae", "ShaeTextEditor")
            .map(|d| d.config_dir().join("config.toml"))
    }

    pub fn load() -> Config {
        let Some(path) = Self::path() else {
            return Config::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let s = toml::to_string_pretty(self).context("serialize config")?;
        std::fs::write(&path, s).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}