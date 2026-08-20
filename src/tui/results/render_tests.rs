use super::app::ResultsApp;
use super::filter::{CoverageFilter, Filter};
use super::sort::SortCriteria;
use crate::priority::unified_scorer::{Location, UnifiedScore};
use crate::priority::{
    ActionableRecommendation, CallGraph, DebtType, FunctionRole, ImpactMetrics, UnifiedAnalysis,
    UnifiedDebtItem,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render(app: &mut ResultsApp) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| app.render(frame))
        .expect("results explorer should render");

    terminal
        .backend()
        .buffer()
        .content()
        .chunks(WIDTH as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_item(file: &str, function: &str, line: usize, score: f64) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: file.into(),
            function: function.into(),
            line,
        },
        debt_type: DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 25,
        },
        unified_score: UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 7.0,
            dependency_factor: 3.0,
            role_multiplier: 1.2,
            final_score: score,
            base_score: Some(25.0),
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: Some(0.7),
            refactorability_factor: Some(1.0),
            pattern_factor: Some(0.85),
            debt_adjustment: None,
            pre_normalization_score: None,
            structural_multiplier: Some(1.15),
            has_coverage_data: false,
            contextual_risk_multiplier: None,
            pre_contextual_score: None,
            debt_type_multiplier: None,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Refactor".into(),
            rationale: "Test finding".into(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            complexity_reduction: 5.0,
            coverage_improvement: 0.1,
            lines_reduction: 10,
            risk_reduction: 0.2,
        },
        transitive_coverage: None,
        file_context: None,
        upstream_dependencies: 5,
        downstream_dependencies: 10,
        upstream_callers: vec![],
        downstream_callees: vec![],
        upstream_production_callers: vec![],
        upstream_test_callers: vec![],
        production_blast_radius: 0,
        nesting_depth: 3,
        function_length: 100,
        cyclomatic_complexity: 15,
        cognitive_complexity: 25,
        is_pure: Some(true),
        purity_confidence: Some(0.9),
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: Some(0.9),
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: None,
        file_line_count: None,
        responsibility_category: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
        context_suggestion: None,
    }
}

fn grouped_app() -> ResultsApp {
    let mut analysis = UnifiedAnalysis::new(CallGraph::new());
    analysis
        .items
        .push(test_item("grouped.rs", "work", 10, 11.0));

    let mut testing_gap = test_item("grouped.rs", "work", 10, 73.0);
    testing_gap.debt_type = DebtType::TestingGap {
        coverage: 0.0,
        cyclomatic: 15,
        cognitive: 25,
    };
    analysis.items.push(testing_gap);
    analysis.items.push(test_item("other.rs", "other", 20, 5.0));
    ResultsApp::new(analysis)
}

#[test]
fn score_breakdown_tracks_grouped_finding_navigation() {
    let mut app = grouped_app();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    app.handle_key(key(KeyCode::Char('2'))).unwrap();

    let first = render(&mut app);
    assert!(first.contains("Finding 1/2"));
    assert!(first.contains("viewing item 1 of 2: High Complexity"));
    assert!(first.contains("11.0"));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    let second = render(&mut app);
    assert!(second.contains("Finding 2/2"));
    assert!(second.contains("viewing item 2 of 2: Testing Gap"));
    assert!(second.contains("73.0"));
    assert_ne!(first, second);

    app.handle_key(key(KeyCode::Char('['))).unwrap();
    let wrapped_back = render(&mut app);
    assert!(wrapped_back.contains("Finding 1/2"));
    assert!(wrapped_back.contains("viewing item 1 of 2: High Complexity"));
    assert!(wrapped_back.contains("11.0"));
}

#[test]
fn detail_help_fits_group_navigation_and_version_at_80_by_24() {
    let mut app = grouped_app();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    app.handle_key(key(KeyCode::Char('?'))).unwrap();

    let output = render(&mut app);
    assert!(output.contains("Detail View"));
    assert!(output.contains("[ / ]"));
    assert!(output.contains("Previous/next finding"));
    assert!(output.contains("Copy current detail page"));
    assert!(output.contains("Open finding in editor"));
    assert!(output.contains("Press any key to close"));
    assert!(output.contains(&format!("debtmap v{}", env!("CARGO_PKG_VERSION"))));
}

#[test]
fn list_help_shows_list_controls_only_at_80_by_24() {
    let mut app = grouped_app();
    app.handle_key(key(KeyCode::Char('?'))).unwrap();

    let output = render(&mut app);
    assert!(output.contains("Move selection"));
    assert!(output.contains("Search"));
    assert!(output.contains("Filter"));
    assert!(output.contains("Group"));
    assert!(output.contains("Open in editor"));
    assert!(output.contains("Press any key to close"));
    assert!(output.contains(&format!("debtmap v{} results", env!("CARGO_PKG_VERSION"))));
    assert!(!output.contains("Previous/next finding"));
    assert!(!output.contains("Scroll half page"));
}

#[test]
fn grouped_finding_resets_after_location_and_query_changes() {
    let mut app = grouped_app();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    assert_eq!(app.selected_group_item_position(), Some((1, 2)));

    app.handle_key(key(KeyCode::Char('j'))).unwrap();
    app.handle_key(key(KeyCode::Char('k'))).unwrap();
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.add_filter(Filter::Coverage(CoverageFilter::None));
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.clear_filters();
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.query_mut()
        .search_mut()
        .set_query("grouped".to_string());
    app.apply_search();
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.set_sort_by(SortCriteria::FilePath);
    assert_eq!(app.nav().detail_group_item_index(), 0);

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.handle_key(key(KeyCode::Esc)).unwrap();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.handle_key(key(KeyCode::Esc)).unwrap();
    app.handle_key(key(KeyCode::Char('u'))).unwrap();
    assert_eq!(app.nav().detail_group_item_index(), 0);
    app.handle_key(key(KeyCode::Char('u'))).unwrap();
    app.handle_key(key(KeyCode::Enter)).unwrap();
    assert_eq!(app.selected_group_item_position(), Some((0, 2)));

    app.handle_key(key(KeyCode::Char(']'))).unwrap();
    app.handle_key(key(KeyCode::Esc)).unwrap();
    app.handle_key(key(KeyCode::Char('G'))).unwrap();
    assert_eq!(app.nav().detail_group_item_index(), 0);
}
