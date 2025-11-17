//! Integration test for state machine pattern detection
//!
//! Verifies that the state machine and coordinator patterns are correctly
//! detected from real Rust code examples.

use debtmap::analyzers::Analyzer;
use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::core::LanguageSpecificData;
use std::path::PathBuf;

#[test]
fn test_reconcile_state_detects_state_machine_or_coordinator() {
    let code = r#"
enum Mode {
    Active,
    Standby,
    Maintenance,
}

enum Action {
    DrainConnections,
    WaitForDrain,
    TransitionToStandby,
    Warmup,
    TransitionToActive,
    AbortPending,
    FinishPending,
    TransitionToMaintenance,
}

struct State {
    mode: Mode,
}

impl State {
    fn has_active_connections(&self) -> bool { true }
    fn requires_warmup(&self) -> bool { true }
    fn has_pending_operations(&self) -> bool { true }
}

fn reconcile_state(current: &State, desired: &State, force_maintenance: bool) -> Vec<Action> {
    let mut actions = vec![];

    match (current.mode, desired.mode) {
        (Mode::Active, Mode::Standby) => {
            if current.has_active_connections() {
                actions.push(Action::DrainConnections);
                actions.push(Action::WaitForDrain);
            }
            actions.push(Action::TransitionToStandby);
        }
        (Mode::Standby, Mode::Active) => {
            if desired.requires_warmup() {
                actions.push(Action::Warmup);
            }
            actions.push(Action::TransitionToActive);
        }
        (Mode::Active, Mode::Maintenance) => {
            if current.has_pending_operations() {
                if force_maintenance {
                    actions.push(Action::AbortPending);
                } else {
                    actions.push(Action::FinishPending);
                }
            }
            actions.push(Action::TransitionToMaintenance);
        }
        _ => {}
    }

    actions
}
"#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Find the reconcile_state function
    let reconcile_fn = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "reconcile_state")
        .expect("reconcile_state function should be found");

    // Check that language-specific data was populated
    assert!(
        reconcile_fn.language_specific.is_some(),
        "Language-specific data should be populated"
    );

    // Extract the Rust pattern data
    if let Some(LanguageSpecificData::Rust(rust_patterns)) = &reconcile_fn.language_specific {
        // Either state machine signals or coordinator signals should be detected
        let has_state_signals = rust_patterns.state_machine_signals.is_some();
        let has_coordinator_signals = rust_patterns.coordinator_signals.is_some();

        assert!(
            has_state_signals || has_coordinator_signals,
            "Either state machine or coordinator pattern should be detected. \
             State signals: {:?}, Coordinator signals: {:?}",
            rust_patterns.state_machine_signals,
            rust_patterns.coordinator_signals
        );

        // If coordinator signals detected, verify the counts
        if let Some(coord_signals) = &rust_patterns.coordinator_signals {
            assert!(
                coord_signals.actions >= 3,
                "Should have at least 3 actions, got {}",
                coord_signals.actions
            );
            assert!(
                coord_signals.comparisons >= 1,
                "Should have at least 1 comparison, got {}",
                coord_signals.comparisons
            );
            assert!(
                coord_signals.has_action_accumulation,
                "Should detect action accumulation"
            );
        }

        // If state machine signals detected, verify the transition count
        if let Some(state_signals) = &rust_patterns.state_machine_signals {
            assert!(
                state_signals.has_enum_match,
                "Should detect enum match"
            );
            assert!(
                state_signals.transition_count >= 2,
                "Should have at least 2 transitions, got {}",
                state_signals.transition_count
            );
        }
    } else {
        panic!("Expected Rust language-specific data");
    }

    // Verify complexity metrics
    assert!(
        reconcile_fn.cyclomatic >= 6,
        "reconcile_state should have cyclomatic complexity >= 6, got {}",
        reconcile_fn.cyclomatic
    );
}

#[test]
fn test_simple_coordinator_pattern() {
    let code = r#"
enum Action { A, B, C }

fn coordinate(x: i32, y: i32) -> Vec<Action> {
    let mut actions = vec![];
    if x > 10 {
        actions.push(Action::A);
    }
    if y < 5 {
        actions.push(Action::B);
    }
    if x + y > 15 {
        actions.push(Action::C);
    }
    actions
}
"#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    let coord_fn = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "coordinate")
        .expect("coordinate function should be found");

    if let Some(LanguageSpecificData::Rust(rust_patterns)) = &coord_fn.language_specific {
        assert!(
            rust_patterns.coordinator_signals.is_some(),
            "Coordinator pattern should be detected"
        );

        if let Some(signals) = &rust_patterns.coordinator_signals {
            assert_eq!(signals.actions, 3, "Should detect 3 action pushes");
            assert!(signals.comparisons >= 3, "Should detect comparisons");
        }
    }
}
