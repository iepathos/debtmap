use crate::config::CallerCalleeConfig;
use colored::*;
use std::env;
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,   // Detect based on terminal
    Always, // Force colors on
    Never,  // Force colors off
}

impl ColorMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            _ => None,
        }
    }

    pub fn should_use_color(&self) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => detect_color_support(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FormattingConfig {
    pub color: ColorMode,
    pub caller_callee: CallerCalleeConfig,
    /// Show detailed module split recommendations for god objects (Spec 208)
    pub show_splits: bool,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            color: ColorMode::Auto,
            caller_callee: CallerCalleeConfig::default(),
            show_splits: false,
        }
    }
}

impl FormattingConfig {
    pub fn new(color: ColorMode) -> Self {
        Self {
            color,
            caller_callee: CallerCalleeConfig::default(),
            show_splits: false,
        }
    }

    pub fn with_caller_callee(color: ColorMode, caller_callee: CallerCalleeConfig) -> Self {
        Self {
            color,
            caller_callee,
            show_splits: false,
        }
    }

    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Check NO_COLOR environment variable (per no-color.org standard)
        if env::var("NO_COLOR").is_ok() {
            config.color = ColorMode::Never;
        }

        // Check CLICOLOR environment variable
        if let Ok(val) = env::var("CLICOLOR") {
            if val == "0" {
                config.color = ColorMode::Never;
            }
        }

        // Check CLICOLOR_FORCE environment variable
        if let Ok(val) = env::var("CLICOLOR_FORCE") {
            if val == "1" {
                config.color = ColorMode::Always;
            }
        }

        config
    }

    /// Create a plain output configuration (no colors)
    pub fn plain() -> Self {
        Self {
            color: ColorMode::Never,
            caller_callee: CallerCalleeConfig::default(),
            show_splits: false,
        }
    }

    /// Create a configuration with show_splits enabled
    pub fn with_show_splits(mut self, show_splits: bool) -> Self {
        self.show_splits = show_splits;
        self
    }
}

pub trait OutputFormatter {
    fn success(&self, text: &str) -> String;
    fn error(&self, text: &str) -> String;
    fn warning(&self, text: &str) -> String;
    fn info(&self, text: &str) -> String;
    fn header(&self, text: &str) -> String;
    fn bold(&self, text: &str) -> String;
    fn dim(&self, text: &str) -> String;
}

pub struct ColoredFormatter {
    config: FormattingConfig,
}

impl ColoredFormatter {
    pub fn new(config: FormattingConfig) -> Self {
        // Set colored control based on configuration
        if config.color.should_use_color() {
            colored::control::set_override(true);
        } else {
            colored::control::set_override(false);
        }

        Self { config }
    }
}

impl OutputFormatter for ColoredFormatter {
    fn success(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.green().to_string()
        } else {
            text.to_string()
        }
    }

    fn error(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.red().to_string()
        } else {
            text.to_string()
        }
    }

    fn warning(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.yellow().to_string()
        } else {
            text.to_string()
        }
    }

    fn info(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.cyan().to_string()
        } else {
            text.to_string()
        }
    }

    fn header(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.blue().bold().to_string()
        } else {
            text.to_string()
        }
    }

    fn bold(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.bold().to_string()
        } else {
            text.to_string()
        }
    }

    fn dim(&self, text: &str) -> String {
        if self.config.color.should_use_color() {
            text.dimmed().to_string()
        } else {
            text.to_string()
        }
    }
}

pub struct PlainFormatter;

impl OutputFormatter for PlainFormatter {
    fn success(&self, text: &str) -> String {
        text.to_string()
    }

    fn error(&self, text: &str) -> String {
        text.to_string()
    }

    fn warning(&self, text: &str) -> String {
        text.to_string()
    }

    fn info(&self, text: &str) -> String {
        text.to_string()
    }

    fn header(&self, text: &str) -> String {
        text.to_string()
    }

    fn bold(&self, text: &str) -> String {
        text.to_string()
    }

    fn dim(&self, text: &str) -> String {
        text.to_string()
    }
}

fn detect_color_support() -> bool {
    // Check if we're in a dumb terminal
    if let Ok(term) = env::var("TERM") {
        if term == "dumb" {
            return false;
        }
    }

    // Check if stdout is a TTY
    std::io::stdout().is_terminal()
}
