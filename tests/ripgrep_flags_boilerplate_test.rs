/// Test that reproduces the ripgrep flags/defs.rs pattern
///
/// The actual pattern in ripgrep is:
/// - 888 separate structs (one per flag)
/// - Each struct implements the same Flag trait
/// - Very uniform implementations (mostly just returning static strings)
/// - High line count (7775 lines) but low complexity
///
/// This should be detected as boilerplate and recommend macro usage,
/// NOT recommend splitting into modules.
use debtmap::organization::boilerplate_detector::BoilerplateDetector;
use std::path::Path;

#[test]
fn test_ripgrep_flags_pattern_detection() {
    // Simplified version of ripgrep's pattern:
    // Many structs, each implementing the same trait with similar methods
    let code = r#"
        // Simulating ripgrep's Flag trait pattern
        pub trait Flag {
            fn name_long(&self) -> &'static str;
            fn name_short(&self) -> Option<char>;
            fn is_switch(&self) -> bool;
            fn doc_category(&self) -> &'static str;
            fn doc_short(&self) -> &'static str;
        }

        // Struct 1
        pub struct AfterContextFlag;
        impl Flag for AfterContextFlag {
            fn name_long(&self) -> &'static str { "after-context" }
            fn name_short(&self) -> Option<char> { Some('A') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Show NUM lines after each match" }
        }

        // Struct 2
        pub struct BeforeContextFlag;
        impl Flag for BeforeContextFlag {
            fn name_long(&self) -> &'static str { "before-context" }
            fn name_short(&self) -> Option<char> { Some('B') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Show NUM lines before each match" }
        }

        // Struct 3
        pub struct ContextFlag;
        impl Flag for ContextFlag {
            fn name_long(&self) -> &'static str { "context" }
            fn name_short(&self) -> Option<char> { Some('C') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Show NUM lines before and after each match" }
        }

        // Struct 4
        pub struct CountFlag;
        impl Flag for CountFlag {
            fn name_long(&self) -> &'static str { "count" }
            fn name_short(&self) -> Option<char> { Some('c') }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Only show count of matching lines" }
        }

        // Struct 5
        pub struct ColorFlag;
        impl Flag for ColorFlag {
            fn name_long(&self) -> &'static str { "color" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "When to use color in output" }
        }

        // Struct 6
        pub struct ColorsFlag;
        impl Flag for ColorsFlag {
            fn name_long(&self) -> &'static str { "colors" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Configure color settings" }
        }

        // Struct 7
        pub struct ColumnFlag;
        impl Flag for ColumnFlag {
            fn name_long(&self) -> &'static str { "column" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Show column number of match" }
        }

        // Struct 8
        pub struct DebugFlag;
        impl Flag for DebugFlag {
            fn name_long(&self) -> &'static str { "debug" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "other" }
            fn doc_short(&self) -> &'static str { "Show debug information" }
        }

        // Struct 9
        pub struct DfaDfaFlag;
        impl Flag for DfaDfaFlag {
            fn name_long(&self) -> &'static str { "dfa-size-limit" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "search" }
            fn doc_short(&self) -> &'static str { "DFA size limit" }
        }

        // Struct 10
        pub struct EncodingFlag;
        impl Flag for EncodingFlag {
            fn name_long(&self) -> &'static str { "encoding" }
            fn name_short(&self) -> Option<char> { Some('E') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "search" }
            fn doc_short(&self) -> &'static str { "Text encoding to use" }
        }

        // Struct 11
        pub struct FileFlag;
        impl Flag for FileFlag {
            fn name_long(&self) -> &'static str { "file" }
            fn name_short(&self) -> Option<char> { Some('f') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "input" }
            fn doc_short(&self) -> &'static str { "Search from file" }
        }

        // Struct 12
        pub struct FilesFlag;
        impl Flag for FilesFlag {
            fn name_long(&self) -> &'static str { "files" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "input" }
            fn doc_short(&self) -> &'static str { "Print files to search" }
        }

        // Struct 13
        pub struct FilesWithMatchesFlag;
        impl Flag for FilesWithMatchesFlag {
            fn name_long(&self) -> &'static str { "files-with-matches" }
            fn name_short(&self) -> Option<char> { Some('l') }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Only print filenames with matches" }
        }

        // Struct 14
        pub struct FilesWithoutMatchFlag;
        impl Flag for FilesWithoutMatchFlag {
            fn name_long(&self) -> &'static str { "files-without-match" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Only print filenames without matches" }
        }

        // Struct 15
        pub struct FixedStringsFlag;
        impl Flag for FixedStringsFlag {
            fn name_long(&self) -> &'static str { "fixed-strings" }
            fn name_short(&self) -> Option<char> { Some('F') }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "search" }
            fn doc_short(&self) -> &'static str { "Treat pattern as literal string" }
        }

        // Struct 16
        pub struct FollowFlag;
        impl Flag for FollowFlag {
            fn name_long(&self) -> &'static str { "follow" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "input" }
            fn doc_short(&self) -> &'static str { "Follow symbolic links" }
        }

        // Struct 17
        pub struct GlobFlag;
        impl Flag for GlobFlag {
            fn name_long(&self) -> &'static str { "glob" }
            fn name_short(&self) -> Option<char> { Some('g') }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "filter" }
            fn doc_short(&self) -> &'static str { "Include/exclude files matching glob" }
        }

        // Struct 18
        pub struct HeadingFlag;
        impl Flag for HeadingFlag {
            fn name_long(&self) -> &'static str { "heading" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Print filename above matches" }
        }

        // Struct 19
        pub struct HiddenFlag;
        impl Flag for HiddenFlag {
            fn name_long(&self) -> &'static str { "hidden" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "filter" }
            fn doc_short(&self) -> &'static str { "Search hidden files and directories" }
        }

        // Struct 20
        pub struct IgnoreCaseFlag;
        impl Flag for IgnoreCaseFlag {
            fn name_long(&self) -> &'static str { "ignore-case" }
            fn name_short(&self) -> Option<char> { Some('i') }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "search" }
            fn doc_short(&self) -> &'static str { "Case insensitive search" }
        }

        // Struct 21
        pub struct IgnoreFileFlag;
        impl Flag for IgnoreFileFlag {
            fn name_long(&self) -> &'static str { "ignore-file" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { false }
            fn doc_category(&self) -> &'static str { "filter" }
            fn doc_short(&self) -> &'static str { "Specify additional ignore file" }
        }

        // Struct 22
        pub struct IncludeZeroFlag;
        impl Flag for IncludeZeroFlag {
            fn name_long(&self) -> &'static str { "include-zero" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Include files with zero matches" }
        }

        // Struct 23
        pub struct InvertMatchFlag;
        impl Flag for InvertMatchFlag {
            fn name_long(&self) -> &'static str { "invert-match" }
            fn name_short(&self) -> Option<char> { Some('v') }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "search" }
            fn doc_short(&self) -> &'static str { "Invert matching" }
        }

        // Struct 24
        pub struct JsonFlag;
        impl Flag for JsonFlag {
            fn name_long(&self) -> &'static str { "json" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Output results as JSON" }
        }

        // Struct 25
        pub struct LineBufferedFlag;
        impl Flag for LineBufferedFlag {
            fn name_long(&self) -> &'static str { "line-buffered" }
            fn name_short(&self) -> Option<char> { None }
            fn is_switch(&self) -> bool { true }
            fn doc_category(&self) -> &'static str { "output" }
            fn doc_short(&self) -> &'static str { "Force line buffering" }
        }
    "#;

    // Parse the code
    let syntax = syn::parse_file(code).expect("Failed to parse ripgrep-style code");

    // Use default detector configuration
    let detector = BoilerplateDetector::default();

    // Analyze the file
    let result = detector.detect(Path::new("flags_defs.rs"), &syntax);

    // Debug output
    println!("\n=== RIPGREP FLAGS PATTERN DETECTION ===");
    println!("Is boilerplate: {}", result.is_boilerplate);
    println!("Confidence: {:.1}%", result.confidence * 100.0);
    println!("Pattern type: {:?}", result.pattern_type);
    println!("Signals detected: {:?}", result.signals);
    println!("\n=== RECOMMENDATION ===");
    if !result.recommendation.is_empty() {
        println!("{}", result.recommendation);
    } else {
        println!("(No recommendation generated)");
    }
    println!("=====================================\n");

    // ASSERTION: This SHOULD be detected as boilerplate
    // We have 25 implementations of the Flag trait with:
    // - Very uniform structure (all implement the same 5 methods)
    // - Low complexity (just return static strings)
    // - High method uniformity (all methods have same signature)
    assert!(
        result.is_boilerplate,
        "Ripgrep-style flag implementations should be detected as boilerplate. \
         Got confidence: {:.1}%, signals: {:?}",
        result.confidence * 100.0,
        result.signals
    );

    // Verify confidence is high (>70%)
    assert!(
        result.confidence >= 0.7,
        "Boilerplate confidence should be >= 70% for this pattern (got {:.1}%)",
        result.confidence * 100.0
    );

    // Verify we got a macro recommendation
    assert!(
        !result.recommendation.is_empty(),
        "Should generate macro recommendation for flag boilerplate"
    );

    assert!(
        result.recommendation.contains("BOILERPLATE DETECTED"),
        "Recommendation should mention boilerplate detection"
    );

    assert!(
        result
            .recommendation
            .contains("NOT a god object requiring module splitting"),
        "Recommendation should clarify this is NOT a module splitting case"
    );

    assert!(
        result.recommendation.contains("macro"),
        "Recommendation should mention using macros"
    );
}

#[test]
fn test_actual_ripgrep_file_if_available() {
    // Try to analyze the actual ripgrep file if it's available
    let ripgrep_path = Path::new("../ripgrep/crates/core/flags/defs.rs");

    if !ripgrep_path.exists() {
        println!("Skipping test - ripgrep source not found at {:?}", ripgrep_path);
        return;
    }

    println!("\n=== ANALYZING ACTUAL RIPGREP FLAGS/DEFS.RS ===");

    // Read and parse the actual file
    let content = std::fs::read_to_string(ripgrep_path)
        .expect("Failed to read ripgrep flags/defs.rs");

    let syntax = syn::parse_file(&content)
        .expect("Failed to parse ripgrep flags/defs.rs");

    // Use default detector
    let detector = BoilerplateDetector::default();
    let result = detector.detect(ripgrep_path, &syntax);

    println!("File: {:?}", ripgrep_path);
    println!("Is boilerplate: {}", result.is_boilerplate);
    println!("Confidence: {:.1}%", result.confidence * 100.0);
    println!("Signals: {:?}", result.signals);

    if result.is_boilerplate {
        println!("\n=== RECOMMENDATION ===");
        println!("{}", result.recommendation);
        println!("=====================\n");

        // This is the expected outcome
        assert!(result.confidence >= 0.7);
        assert!(result.recommendation.contains("macro"));
    } else {
        println!("\nWARNING: Actual ripgrep file NOT detected as boilerplate!");
        println!("This indicates the boilerplate detector needs tuning.");
        println!("Confidence was only: {:.1}%", result.confidence * 100.0);

        // This is a failure - we should detect ripgrep's flags as boilerplate
        panic!(
            "FAIL: ripgrep flags/defs.rs should be detected as boilerplate pattern. \
             Got confidence: {:.1}%, threshold is 70%",
            result.confidence * 100.0
        );
    }
}
