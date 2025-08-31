use debtmap::output::formatters::determine_priority_output_format;
use debtmap::priority::formatter::OutputFormat;

#[test]
fn test_determine_priority_output_format_top() {
    let format = determine_priority_output_format(Some(5), None);
    assert!(matches!(format, OutputFormat::Top(5)));

    let format = determine_priority_output_format(Some(10), None);
    assert!(matches!(format, OutputFormat::Top(10)));

    let format = determine_priority_output_format(Some(1), None);
    assert!(matches!(format, OutputFormat::Top(1)));
}

#[test]
fn test_determine_priority_output_format_default() {
    let format = determine_priority_output_format(None, None);
    assert!(matches!(format, OutputFormat::Default));
}

#[test]
fn test_determine_priority_output_format_precedence_order() {
    // Test precedence: tail > top > default
    let format = determine_priority_output_format(Some(5), None);
    assert!(matches!(format, OutputFormat::Top(5)));

    let format = determine_priority_output_format(None, None);
    assert!(matches!(format, OutputFormat::Default));

    let format = determine_priority_output_format(None, Some(3));
    assert!(matches!(format, OutputFormat::Tail(3)));

    // tail takes precedence over top
    let format = determine_priority_output_format(Some(5), Some(3));
    assert!(matches!(format, OutputFormat::Tail(3)));
}

#[test]
fn test_determine_priority_output_format_tail() {
    // tail should work when specified alone
    let format = determine_priority_output_format(None, Some(5));
    assert!(matches!(format, OutputFormat::Tail(5)));

    // tail with different values
    let format = determine_priority_output_format(None, Some(10));
    assert!(matches!(format, OutputFormat::Tail(10)));

    let format = determine_priority_output_format(None, Some(1));
    assert!(matches!(format, OutputFormat::Tail(1)));
}
