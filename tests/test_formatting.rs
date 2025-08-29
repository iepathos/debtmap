use debtmap::formatting::{
    ColorMode, ColoredFormatter, EmojiMode, FormattingConfig, OutputFormatter,
};

#[test]
fn test_color_mode_from_str() {
    assert_eq!(ColorMode::parse("auto"), Some(ColorMode::Auto));
    assert_eq!(ColorMode::parse("always"), Some(ColorMode::Always));
    assert_eq!(ColorMode::parse("never"), Some(ColorMode::Never));
    assert_eq!(ColorMode::parse("NEVER"), Some(ColorMode::Never));
    assert_eq!(ColorMode::parse("invalid"), None);
}

#[test]
fn test_emoji_mode_from_str() {
    assert_eq!(EmojiMode::parse("auto"), Some(EmojiMode::Auto));
    assert_eq!(EmojiMode::parse("always"), Some(EmojiMode::Always));
    assert_eq!(EmojiMode::parse("never"), Some(EmojiMode::Never));
    assert_eq!(EmojiMode::parse("NEVER"), Some(EmojiMode::Never));
    assert_eq!(EmojiMode::parse("invalid"), None);
}

#[test]
fn test_formatting_config_from_env() {
    // Test with NO_COLOR set
    std::env::set_var("NO_COLOR", "1");
    let config = FormattingConfig::from_env();
    assert_eq!(config.color, ColorMode::Never);
    std::env::remove_var("NO_COLOR");

    // Test with CLICOLOR=0
    std::env::set_var("CLICOLOR", "0");
    let config = FormattingConfig::from_env();
    assert_eq!(config.color, ColorMode::Never);
    std::env::remove_var("CLICOLOR");

    // Test with CLICOLOR_FORCE=1
    std::env::set_var("CLICOLOR_FORCE", "1");
    let config = FormattingConfig::from_env();
    assert_eq!(config.color, ColorMode::Always);
    std::env::remove_var("CLICOLOR_FORCE");
}

#[test]
fn test_color_mode_should_use_color() {
    assert_eq!(ColorMode::Always.should_use_color(), true);
    assert_eq!(ColorMode::Never.should_use_color(), false);
    // Auto depends on terminal detection, but we can test it exists
    let _ = ColorMode::Auto.should_use_color();
}

#[test]
fn test_colored_formatter_with_color() {
    let config = FormattingConfig::new(ColorMode::Always, EmojiMode::Always);
    let formatter = ColoredFormatter::new(config);

    // These will include ANSI codes when color is enabled
    let success = formatter.success("test");
    let error = formatter.error("test");
    let warning = formatter.warning("test");
    let info = formatter.info("test");

    // We can't easily test the actual ANSI codes without depending on colored internals
    // but we can verify the methods work
    assert!(success.contains("test"));
    assert!(error.contains("test"));
    assert!(warning.contains("test"));
    assert!(info.contains("test"));
}

#[test]
fn test_colored_formatter_without_color() {
    let config = FormattingConfig::new(ColorMode::Never, EmojiMode::Never);
    let formatter = ColoredFormatter::new(config);

    // Without color, these should just return the plain text
    assert_eq!(formatter.success("test"), "test");
    assert_eq!(formatter.error("test"), "test");
    assert_eq!(formatter.warning("test"), "test");
    assert_eq!(formatter.info("test"), "test");
    assert_eq!(formatter.bold("test"), "test");
    assert_eq!(formatter.dim("test"), "test");
}

#[test]
fn test_emoji_formatting() {
    let config_with_emoji = FormattingConfig::new(ColorMode::Never, EmojiMode::Always);
    let formatter_with = ColoredFormatter::new(config_with_emoji);

    let config_without_emoji = FormattingConfig::new(ColorMode::Never, EmojiMode::Never);
    let formatter_without = ColoredFormatter::new(config_without_emoji);

    assert_eq!(formatter_with.emoji("✓", "[OK]"), "✓");
    assert_eq!(formatter_without.emoji("✓", "[OK]"), "[OK]");

    assert_eq!(formatter_with.emoji("📊", "[STATS]"), "📊");
    assert_eq!(formatter_without.emoji("📊", "[STATS]"), "[STATS]");
}
