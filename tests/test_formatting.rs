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
    assert!(ColorMode::Always.should_use_color());
    assert!(!ColorMode::Never.should_use_color());
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

#[test]
fn test_plain_output_mode_is_ascii_only() {
    // Create a formatter with plain mode settings (no color, no emoji)
    let config = FormattingConfig::plain();
    let formatter = ColoredFormatter::new(config);

    // Test that colors are disabled
    assert_eq!(config.color, ColorMode::Never);
    assert_eq!(config.emoji, EmojiMode::Never);

    // Test that all text formatting returns plain ASCII text
    assert_eq!(formatter.success("SUCCESS"), "SUCCESS");
    assert_eq!(formatter.error("ERROR"), "ERROR");
    assert_eq!(formatter.warning("WARNING"), "WARNING");
    assert_eq!(formatter.info("INFO"), "INFO");
    assert_eq!(formatter.bold("BOLD"), "BOLD");
    assert_eq!(formatter.dim("DIM"), "DIM");

    // Test that emojis are replaced with ASCII alternatives
    assert_eq!(formatter.emoji("🎯", "[TARGET]"), "[TARGET]");
    assert_eq!(formatter.emoji("✅", "[OK]"), "[OK]");
    assert_eq!(formatter.emoji("❌", "[FAIL]"), "[FAIL]");
    assert_eq!(formatter.emoji("⚠️", "[WARN]"), "[WARN]");
    assert_eq!(formatter.emoji("📊", "[STATS]"), "[STATS]");
    assert_eq!(formatter.emoji("🔍", "[SEARCH]"), "[SEARCH]");
    assert_eq!(formatter.emoji("📈", "[GRAPH]"), "[GRAPH]");
    assert_eq!(formatter.emoji("💡", "[TIP]"), "[TIP]");

    // Verify that all output is pure ASCII (no Unicode characters)
    let test_strings = vec![
        formatter.success("test"),
        formatter.error("test"),
        formatter.warning("test"),
        formatter.info("test"),
        formatter.bold("test"),
        formatter.dim("test"),
        formatter.emoji("🎯", "[TARGET]"),
        formatter.emoji("✨", "[FEATURE]"),
    ];

    for s in test_strings {
        assert!(
            s.chars().all(|c| c.is_ascii()),
            "Output '{}' contains non-ASCII characters",
            s
        );
    }
}

#[test]
fn test_plain_mode_complex_formatting() {
    let config = FormattingConfig::plain();
    let formatter = ColoredFormatter::new(config);

    // Test complex nested formatting scenarios
    let complex_text = "Technical Debt Report";

    // In plain mode, all formatting should be stripped
    assert_eq!(formatter.bold(&formatter.error(complex_text)), complex_text);
    assert_eq!(
        formatter.dim(&formatter.warning(complex_text)),
        complex_text
    );
    assert_eq!(
        formatter.success(&format!(
            "{} {}",
            formatter.emoji("✅", "[PASS]"),
            "All tests passed"
        )),
        "[PASS] All tests passed"
    );

    // Test that numeric formatting is preserved
    let stats = "Found 42 issues in 10 files";
    assert_eq!(formatter.info(stats), stats);

    // Test special characters that should remain in ASCII mode
    let special = "Score: 85% | Complexity: 3/10 | Files: src/*.rs";
    assert_eq!(formatter.bold(special), special);
    assert!(special.chars().all(|c| c.is_ascii()));
}
