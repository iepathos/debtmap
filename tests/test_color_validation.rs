use colored::Colorize;

#[test]
fn test_why_text_is_not_dimmed() {
    // Force colored output in test
    colored::control::set_override(true);

    // Test the actual color methods we're using
    let rationale = "This function has high complexity and needs refactoring";

    // Test what each produces
    let dimmed_output = format!("{}", rationale.dimmed());
    let white_output = format!("{}", rationale.white());
    let bright_white_output = format!("{}", rationale.bright_white());

    println!("Testing color outputs:");
    println!("dimmed: {:?}", dimmed_output);
    println!("white: {:?}", white_output);
    println!("bright_white: {:?}", bright_white_output);

    // Check ANSI codes
    assert!(
        dimmed_output.contains("\x1b[2m"),
        "dimmed() should contain \\x1b[2m"
    );
    assert!(
        white_output.contains("\x1b[37m"),
        "white() should contain \\x1b[37m"
    );
    assert!(
        bright_white_output.contains("\x1b[97m"),
        "bright_white() should contain \\x1b[97m"
    );

    // Most importantly: bright_white should NOT contain the dim code
    assert!(
        !bright_white_output.contains("\x1b[2m"),
        "bright_white() should NOT contain dimmed ANSI code \\x1b[2m"
    );

    println!("\n✓ Color methods produce expected ANSI codes");
    println!("✓ bright_white() does not produce dimmed text");
}

#[test]
fn verify_formatter_uses_correct_colors() {
    use std::fs;

    // Read the formatter source files to verify color usage
    let formatter_content =
        fs::read_to_string("src/priority/formatter.rs").expect("Could not read formatter.rs");
    let formatter_verbosity_content = fs::read_to_string("src/priority/formatter_verbosity.rs")
        .expect("Could not read formatter_verbosity.rs");

    // Check that WHY label uses bright_blue and rationale uses no color (plain text)
    // Updated for spec 139: "WHY THIS MATTERS" label format
    let formatter_why_label_blue = formatter_content.contains("WHY THIS MATTERS:\".bright_blue()");
    let formatter_has_dimmed = formatter_content.contains("rationale.dimmed()");
    let formatter_has_bright_white = formatter_content.contains("rationale.bright_white()");

    // Updated for spec 139: "WHY THIS MATTERS" label format in both files
    let verbosity_why_label_blue = formatter_verbosity_content.contains("WHY THIS MATTERS:\".bright_blue()");
    let verbosity_has_dimmed = formatter_verbosity_content.contains("rationale.dimmed()");
    let verbosity_has_bright_white =
        formatter_verbosity_content.contains("rationale.bright_white()");

    println!("Formatter check:");
    println!(
        "  formatter.rs WHY label uses bright_blue: {}",
        formatter_why_label_blue
    );
    println!(
        "  formatter.rs uses dimmed for rationale: {}",
        formatter_has_dimmed
    );
    println!(
        "  formatter.rs uses bright_white for rationale: {}",
        formatter_has_bright_white
    );
    println!(
        "  formatter_verbosity.rs WHY label uses bright_blue: {}",
        verbosity_why_label_blue
    );
    println!(
        "  formatter_verbosity.rs uses dimmed for rationale: {}",
        verbosity_has_dimmed
    );
    println!(
        "  formatter_verbosity.rs uses bright_white for rationale: {}",
        verbosity_has_bright_white
    );

    // Verify correct implementation:
    // - WHY label should use bright_blue
    // - Rationale should NOT use dimmed (hard to read)
    // - Rationale should use plain text (no color modifier) for best readability
    assert!(
        formatter_why_label_blue,
        "formatter.rs should use bright_blue() for WHY label"
    );
    assert!(
        !formatter_has_dimmed,
        "formatter.rs should NOT use dimmed() for rationale"
    );
    assert!(
        !formatter_has_bright_white,
        "formatter.rs should NOT use bright_white() for rationale (appears grey on some terminals)"
    );

    assert!(
        verbosity_why_label_blue,
        "formatter_verbosity.rs should use bright_blue() for WHY label"
    );
    assert!(
        !verbosity_has_dimmed,
        "formatter_verbosity.rs should NOT use dimmed() for rationale"
    );
    assert!(
        !verbosity_has_bright_white,
        "formatter_verbosity.rs should NOT use bright_white() for rationale (appears grey on some terminals)"
    );

    println!(
        "\n✓ Both formatters correctly use bright_blue for labels and plain text for rationale"
    );
}
