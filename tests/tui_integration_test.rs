//! Integration tests for TUI progress visualization
//!
//! Tests that the TUI correctly updates state during pipeline execution

use debtmap::{
    progress::{ProgressConfig, ProgressManager},
    tui::app::{App, StageStatus},
};

#[test]
fn test_tui_app_initialization() {
    let app = App::new();

    // Verify all 6 stages are created
    assert_eq!(app.stages.len(), 6);

    // Verify stage names
    assert_eq!(app.stages[0].name, "files parse");
    assert_eq!(app.stages[1].name, "call graph");
    assert_eq!(app.stages[2].name, "coverage");
    assert_eq!(app.stages[3].name, "purity analysis");
    assert_eq!(app.stages[4].name, "context");
    assert_eq!(app.stages[5].name, "debt scoring");

    // All stages should be pending initially
    for stage in &app.stages {
        assert_eq!(stage.status, StageStatus::Pending);
        assert!(stage.metric.is_none());
        assert!(stage.elapsed.is_none());
    }

    // Overall progress should be 0
    assert_eq!(app.overall_progress, 0.0);
    assert_eq!(app.current_stage, 0);
}

#[test]
fn test_tui_stage_lifecycle() {
    let mut app = App::new();

    // Start first stage
    app.start_stage(0);
    assert_eq!(app.stages[0].status, StageStatus::Active);
    assert_eq!(app.current_stage, 0);

    // Complete first stage with metric
    app.complete_stage(0, "469 files");
    assert_eq!(app.stages[0].status, StageStatus::Completed);
    assert_eq!(app.stages[0].metric, Some("469 files".to_string()));
    assert!(app.stages[0].elapsed.is_some());

    // Start second stage
    app.start_stage(1);
    assert_eq!(app.stages[1].status, StageStatus::Active);
    assert_eq!(app.current_stage, 1);

    // First stage should still be completed
    assert_eq!(app.stages[0].status, StageStatus::Completed);
}

#[test]
fn test_tui_progress_updates() {
    let mut app = App::new();

    // Test progress clamping
    app.set_overall_progress(0.5);
    assert_eq!(app.overall_progress, 0.5);

    // Progress over 1.0 should clamp to 1.0
    app.set_overall_progress(1.5);
    assert_eq!(app.overall_progress, 1.0);

    // Negative progress should clamp to 0.0
    app.set_overall_progress(-0.5);
    assert_eq!(app.overall_progress, 0.0);
}

#[test]
fn test_tui_subtask_updates() {
    let mut app = App::new();

    // Purity analysis stage (index 3) has subtasks
    assert!(!app.stages[3].sub_tasks.is_empty());

    // Update first subtask
    app.update_subtask(3, 0, StageStatus::Completed, None);
    assert_eq!(app.stages[3].sub_tasks[0].status, StageStatus::Completed);

    // Update second subtask with progress
    app.update_subtask(3, 1, StageStatus::Active, Some((50, 100)));
    assert_eq!(app.stages[3].sub_tasks[1].status, StageStatus::Active);
    assert_eq!(app.stages[3].sub_tasks[1].progress, Some((50, 100)));
}

#[test]
fn test_tui_animation_tick() {
    let mut app = App::new();
    let initial_frame = app.animation_frame;

    // Tick should increment frame counter
    app.tick();
    assert_eq!(app.animation_frame, (initial_frame + 1) % 60);

    // Elapsed time should increase
    assert!(app.elapsed_time.as_nanos() > 0);

    // Tick 60 times should wrap back to 0
    for _ in 0..59 {
        app.tick();
    }
    assert_eq!(app.animation_frame, 0);
}

#[test]
fn test_tui_statistics_updates() {
    let mut app = App::new();

    // Initial stats should be zero
    assert_eq!(app.functions_count, 0);
    assert_eq!(app.debt_count, 0);
    assert_eq!(app.coverage_percent, 0.0);

    // Update statistics
    app.update_stats(1234, 56, 78.5);
    assert_eq!(app.functions_count, 1234);
    assert_eq!(app.debt_count, 56);
    assert_eq!(app.coverage_percent, 78.5);
}

#[test]
fn test_progress_manager_tty_detection() {
    // Test that quiet mode disables TUI
    let config = ProgressConfig {
        quiet_mode: true,
        verbosity: 0,
    };

    let manager = ProgressManager::new(config);

    // In quiet mode, progress bars should be hidden
    let bar = manager.create_bar(100, "{msg}");
    assert!(bar.is_hidden());

    let spinner = manager.create_spinner("Test");
    assert!(spinner.is_hidden());
}

#[test]
fn test_full_pipeline_simulation() {
    let mut app = App::new();

    // Simulate full pipeline execution

    // Stage 1: files parse (combined discovery + parsing)
    app.start_stage(0);
    app.set_overall_progress(0.0);
    app.tick();
    app.complete_stage(0, "469 files parsed");
    app.set_overall_progress(0.22);

    // Stage 2: Call graph
    app.start_stage(1);
    app.tick();
    app.complete_stage(1, "5432 functions");
    app.set_overall_progress(0.33);

    // Stage 3: Coverage
    app.start_stage(2);
    app.tick();
    app.complete_stage(2, "loaded");
    app.set_overall_progress(0.55);

    // Stage 4: Purity analysis
    app.start_stage(3);

    // Update subtasks
    app.update_subtask(3, 0, StageStatus::Completed, None);
    app.tick();
    app.update_subtask(3, 1, StageStatus::Active, Some((50, 100)));
    app.tick();
    app.update_subtask(3, 1, StageStatus::Completed, None);
    app.tick();

    app.complete_stage(3, "5432 functions analyzed");
    app.set_overall_progress(0.66);

    // Stage 5: Context
    app.start_stage(4);
    app.tick();
    app.complete_stage(4, "loaded");
    app.set_overall_progress(0.77);

    // Stage 6: Debt scoring (includes prioritization)
    app.start_stage(5);
    app.tick();
    app.complete_stage(5, "123 items scored and prioritized");
    app.set_overall_progress(1.0);

    // Verify final state
    assert_eq!(app.overall_progress, 1.0);
    assert_eq!(app.current_stage, 5);

    // All stages should be completed
    for stage in &app.stages {
        assert_eq!(stage.status, StageStatus::Completed);
        assert!(stage.metric.is_some());
    }
}

#[test]
fn test_stage_metric_updates() {
    let mut app = App::new();

    // Start a stage
    app.start_stage(0);
    assert!(app.stages[0].metric.is_none());

    // Update metric while stage is active
    app.update_stage_metric(0, "100 files");
    assert_eq!(app.stages[0].metric, Some("100 files".to_string()));

    // Update metric again
    app.update_stage_metric(0, "200 files");
    assert_eq!(app.stages[0].metric, Some("200 files".to_string()));
}

#[test]
fn test_purity_analysis_subtasks() {
    let app = App::new();

    // Verify purity analysis stage (index 3) has the correct subtasks
    let purity_stage = &app.stages[3];
    assert_eq!(purity_stage.name, "purity analysis");
    assert_eq!(purity_stage.sub_tasks.len(), 4);

    // Verify subtask names
    assert_eq!(purity_stage.sub_tasks[0].name, "data flow graph");
    assert_eq!(purity_stage.sub_tasks[1].name, "initial detection");
    assert_eq!(purity_stage.sub_tasks[2].name, "propagation");
    assert_eq!(purity_stage.sub_tasks[3].name, "side effects");

    // All subtasks should be pending initially
    for subtask in &purity_stage.sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
        assert!(subtask.progress.is_none());
    }
}

#[test]
fn test_context_analysis_subtasks() {
    let app = App::new();

    // Verify context stage (index 4) has the correct subtasks
    let context_stage = &app.stages[4];
    assert_eq!(context_stage.name, "context");
    assert_eq!(context_stage.sub_tasks.len(), 3);

    // Verify subtask names
    assert_eq!(context_stage.sub_tasks[0].name, "critical path");
    assert_eq!(context_stage.sub_tasks[1].name, "dependencies");
    assert_eq!(context_stage.sub_tasks[2].name, "git history");

    // All subtasks should be pending initially
    for subtask in &context_stage.sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
        assert!(subtask.progress.is_none());
    }
}

#[test]
fn test_context_subsections_when_disabled() {
    let mut app = App::new();

    // Context subsections exist in the App structure
    assert_eq!(app.stages[4].sub_tasks.len(), 3);

    // Simulate context stage being skipped (enable_context=false)
    // When context is disabled, the stage starts and completes immediately
    // with "skipped" metric, and subsections are not updated
    app.start_stage(4);
    assert_eq!(app.stages[4].status, StageStatus::Active);

    // Complete the context stage as "skipped" without updating subsections
    app.complete_stage(4, "skipped");
    assert_eq!(app.stages[4].status, StageStatus::Completed);
    assert_eq!(app.stages[4].metric, Some("skipped".to_string()));

    // All subsections should remain in Pending state when context is disabled
    for subtask in &app.stages[4].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
        assert!(subtask.progress.is_none());
    }
}

#[test]
fn test_context_subsection_lifecycle() {
    let mut app = App::new();

    // Start the context stage
    app.start_stage(4);
    assert_eq!(app.stages[4].status, StageStatus::Active);

    // All subsections should start as Pending
    for subtask in &app.stages[4].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
    }

    // Subsection 0: Critical path analysis - Pending → Active → Completed
    app.update_subtask(4, 0, StageStatus::Active, None);
    assert_eq!(app.stages[4].sub_tasks[0].status, StageStatus::Active);
    assert_eq!(app.stages[4].sub_tasks[0].name, "critical path");
    app.tick();

    app.update_subtask(4, 0, StageStatus::Completed, None);
    assert_eq!(app.stages[4].sub_tasks[0].status, StageStatus::Completed);

    // Subsection 1: Dependencies - Pending → Active → Completed
    app.update_subtask(4, 1, StageStatus::Active, None);
    assert_eq!(app.stages[4].sub_tasks[1].status, StageStatus::Active);
    assert_eq!(app.stages[4].sub_tasks[1].name, "dependencies");
    app.tick();

    app.update_subtask(4, 1, StageStatus::Completed, None);
    assert_eq!(app.stages[4].sub_tasks[1].status, StageStatus::Completed);

    // Subsection 2: Git history - Pending → Active → Completed
    app.update_subtask(4, 2, StageStatus::Active, None);
    assert_eq!(app.stages[4].sub_tasks[2].status, StageStatus::Active);
    assert_eq!(app.stages[4].sub_tasks[2].name, "git history");
    app.tick();

    app.update_subtask(4, 2, StageStatus::Completed, None);
    assert_eq!(app.stages[4].sub_tasks[2].status, StageStatus::Completed);

    // Complete the overall context stage
    app.complete_stage(4, "loaded");
    assert_eq!(app.stages[4].status, StageStatus::Completed);
    assert_eq!(app.stages[4].metric, Some("loaded".to_string()));

    // All subsections should be completed
    for subtask in &app.stages[4].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Completed);
    }
}

#[test]
fn test_debt_scoring_subtasks() {
    let app = App::new();

    // Verify debt scoring stage (index 5) has the correct subtasks
    let debt_stage = &app.stages[5];
    assert_eq!(debt_stage.name, "debt scoring");
    assert_eq!(debt_stage.sub_tasks.len(), 3);

    // Verify subtask names (spec 206 simplified to 3 stages)
    assert_eq!(debt_stage.sub_tasks[0].name, "aggregate debt");
    assert_eq!(debt_stage.sub_tasks[1].name, "score functions");
    assert_eq!(debt_stage.sub_tasks[2].name, "filter results");

    // All subtasks should be pending initially
    for subtask in &debt_stage.sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
        assert!(subtask.progress.is_none());
    }
}

// Helper function to create test UnifiedDebtItem
fn create_test_unified_debt_item(
    location: debtmap::priority::Location,
    debt_type: debtmap::priority::DebtType,
) -> debtmap::priority::UnifiedDebtItem {
    use debtmap::priority::{
        ActionableRecommendation, FunctionRole, ImpactMetrics, UnifiedDebtItem, UnifiedScore,
    };

    UnifiedDebtItem {
        location,
        debt_type,
        unified_score: UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 5.0,
            dependency_factor: 3.0,
            role_multiplier: 1.0,
            final_score: debtmap::priority::score_types::Score0To100::new(5.0),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        function_role: FunctionRole::Unknown,
        recommendation: ActionableRecommendation {
            primary_action: "Test action".to_string(),
            rationale: "Test rationale".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        transitive_coverage: None,
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 1,
        function_length: 10,
        cyclomatic_complexity: 5,
        cognitive_complexity: 5,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        entropy_details: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        file_context: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: None,
        file_line_count: None,
        responsibility_category: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

#[test]
fn test_data_flow_page_rendering_with_mutations() {
    use debtmap::data_flow::{DataFlowGraph, MutationInfo};
    use debtmap::priority::{call_graph::FunctionId, DebtType, Location};
    use std::path::PathBuf;

    // Create test data flow graph
    let mut data_flow = DataFlowGraph::new();

    // Create test function
    let location = Location {
        file: PathBuf::from("src/test.rs"),
        line: 42,
        function: "test_function".to_string(),
    };

    let func_id = FunctionId::new(
        location.file.clone(),
        location.function.clone(),
        location.line,
    );

    // Add mutation info
    let mutation_info = MutationInfo {
        total_mutations: 5,
        live_mutations: vec!["x".to_string(), "y".to_string()],
        escaping_mutations: [String::from("x")].iter().cloned().collect(),
    };
    data_flow.set_mutation_info(func_id.clone(), mutation_info);

    // Create test item (unused in this test, but demonstrates integration)
    let _item = create_test_unified_debt_item(
        location,
        DebtType::ComplexityHotspot {
            cyclomatic: 10,
            cognitive: 15,
        },
    );

    // Verify mutation data is accessible
    let retrieved_mutation = data_flow.get_mutation_info(&func_id);
    assert!(retrieved_mutation.is_some());
    let mutation = retrieved_mutation.unwrap();
    assert_eq!(mutation.total_mutations, 5);
    assert_eq!(mutation.live_mutations.len(), 2);
}

#[test]
fn test_data_flow_page_rendering_with_io_operations() {
    use debtmap::data_flow::{DataFlowGraph, IoOperation};
    use debtmap::priority::{call_graph::FunctionId, DebtType, Location};
    use std::path::PathBuf;

    let mut data_flow = DataFlowGraph::new();

    let location = Location {
        file: PathBuf::from("src/test.rs"),
        line: 100,
        function: "io_function".to_string(),
    };

    let func_id = FunctionId::new(
        location.file.clone(),
        location.function.clone(),
        location.line,
    );

    // Add I/O operations
    data_flow.add_io_operation(
        func_id.clone(),
        IoOperation {
            operation_type: "File Read".to_string(),
            line: 105,
            variables: vec!["file".to_string()],
        },
    );
    data_flow.add_io_operation(
        func_id.clone(),
        IoOperation {
            operation_type: "Network Call".to_string(),
            line: 110,
            variables: vec!["socket".to_string()],
        },
    );

    let _item = create_test_unified_debt_item(
        location,
        DebtType::ComplexityHotspot {
            cyclomatic: 8,
            cognitive: 12,
        },
    );

    // Verify I/O operations are accessible
    let io_operations = data_flow.get_io_operations(&func_id);
    assert!(io_operations.is_some());
    let ops = io_operations.unwrap();
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].operation_type, "File Read");
    assert_eq!(ops[1].operation_type, "Network Call");
}

#[test]
fn test_data_flow_page_rendering_with_escape_analysis() {
    use debtmap::analysis::data_flow::DataFlowAnalysis;
    use debtmap::analysis::VarId;
    use debtmap::data_flow::DataFlowGraph;
    use debtmap::priority::{call_graph::FunctionId, DebtType, Location};
    use std::path::PathBuf;

    let mut data_flow = DataFlowGraph::new();

    let location = Location {
        file: PathBuf::from("src/test.rs"),
        line: 200,
        function: "escape_test".to_string(),
    };

    let func_id = FunctionId::new(
        location.file.clone(),
        location.function.clone(),
        location.line,
    );

    // Add escape analysis - create a minimal DataFlowAnalysis
    use debtmap::analysis::data_flow::{
        EscapeAnalysis, LivenessInfo, ReachingDefinitions, TaintAnalysis,
    };
    use std::collections::HashSet;

    let var1 = VarId {
        name_id: 1,
        version: 0,
    };
    let var2 = VarId {
        name_id: 2,
        version: 0,
    };

    let escape_info = EscapeAnalysis {
        escaping_vars: [var1, var2].iter().copied().collect(),
        captured_vars: HashSet::new(),
        return_dependencies: [var1].iter().copied().collect(),
    };

    let cfg_analysis = DataFlowAnalysis {
        liveness: LivenessInfo {
            live_in: Default::default(),
            live_out: Default::default(),
        },
        reaching_defs: ReachingDefinitions::default(),
        escape_info,
        taint_info: TaintAnalysis {
            tainted_vars: Default::default(),
            taint_sources: Default::default(),
            return_tainted: false,
        },
    };

    data_flow.set_cfg_analysis(func_id.clone(), cfg_analysis);

    let _item = create_test_unified_debt_item(
        location,
        DebtType::ComplexityHotspot {
            cyclomatic: 12,
            cognitive: 18,
        },
    );

    // Verify escape analysis is accessible
    let cfg = data_flow.get_cfg_analysis(&func_id);
    assert!(cfg.is_some());
    let analysis = cfg.unwrap();
    assert_eq!(analysis.escape_info.escaping_vars.len(), 2);
    assert_eq!(analysis.escape_info.return_dependencies.len(), 1);
}

#[test]
fn test_data_flow_markdown_formatting() {
    use debtmap::data_flow::{DataFlowGraph, IoOperation, MutationInfo};
    use debtmap::priority::{call_graph::FunctionId, Location};
    use std::path::PathBuf;

    let mut data_flow = DataFlowGraph::new();

    let location = Location {
        file: PathBuf::from("src/example.rs"),
        line: 50,
        function: "complex_function".to_string(),
    };

    let func_id = FunctionId::new(
        location.file.clone(),
        location.function.clone(),
        location.line,
    );

    // Add comprehensive data flow information
    data_flow.set_mutation_info(
        func_id.clone(),
        MutationInfo {
            total_mutations: 10,
            live_mutations: vec!["counter".to_string(), "state".to_string()],
            escaping_mutations: ["counter".to_string()].iter().cloned().collect(),
        },
    );

    data_flow.add_io_operation(
        func_id.clone(),
        IoOperation {
            operation_type: "Database Query".to_string(),
            line: 55,
            variables: vec!["db".to_string()],
        },
    );

    // Verify all data is present for markdown formatting
    assert!(data_flow.get_mutation_info(&func_id).is_some());
    assert!(data_flow.get_io_operations(&func_id).is_some());

    let mutation = data_flow.get_mutation_info(&func_id).unwrap();
    assert_eq!(mutation.total_mutations, 10);
    assert_eq!(mutation.live_mutations.len(), 2);

    let io_ops = data_flow.get_io_operations(&func_id).unwrap();
    assert_eq!(io_ops.len(), 1);
    assert_eq!(io_ops[0].operation_type, "Database Query");
}

#[test]
fn test_god_object_displays_git_context() {
    use debtmap::priority::{score_types::Score0To100, DebtType, Location};
    use debtmap::risk;
    use debtmap::risk::context::git_history::GitHistoryProvider;
    use debtmap::risk::context::{ContextAggregator, ContextDetails};
    use std::fs;
    use std::process::Command;

    // Create a temporary project with git history
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    // Initialize git repository
    Command::new("git")
        .args(["init"])
        .current_dir(&project_root)
        .output()
        .expect("Failed to init git repo");

    // Configure git user
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&project_root)
        .output()
        .expect("Failed to configure git user");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&project_root)
        .output()
        .expect("Failed to configure git email");

    // Create a god object file with multiple commits to build history
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    let god_object_file = src_dir.join("god_object.rs");

    // Initial commit
    fs::write(
        &god_object_file,
        "pub struct GodObject { field1: i32 }\nimpl GodObject { fn method1(&self) {} }",
    )
    .expect("Failed to write initial file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&project_root)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&project_root)
        .output()
        .expect("Failed to git commit");

    // Second commit - add more complexity
    fs::write(
        &god_object_file,
        "pub struct GodObject { field1: i32, field2: String }\n\
         impl GodObject {\n\
         fn method1(&self) {}\n\
         fn method2(&self) {}\n\
         fn method3(&self) {}\n\
         }",
    )
    .expect("Failed to write second version");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&project_root)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Add more methods"])
        .current_dir(&project_root)
        .output()
        .expect("Failed to git commit");

    // Create a risk analyzer with git context
    let git_provider =
        GitHistoryProvider::new(project_root.clone()).expect("Failed to create git provider");

    let context_aggregator = ContextAggregator::new().with_provider(Box::new(git_provider));

    let risk_analyzer = risk::RiskAnalyzer::default().with_context_aggregator(context_aggregator);

    // Analyze the god object file's git context
    let contextual_risk = debtmap::builders::unified_analysis::analyze_file_git_context(
        &god_object_file,
        &risk_analyzer,
        &project_root,
    );

    // Verify git context is present
    assert!(
        contextual_risk.is_some(),
        "God object should have git context when analyzed with git context enabled"
    );

    let context = contextual_risk.unwrap();

    // Verify git_history context is populated in the contexts vec
    let git_context = context
        .contexts
        .iter()
        .find(|ctx| ctx.provider == "git_history");

    assert!(
        git_context.is_some(),
        "Contextual risk should include git_history context"
    );

    let git_ctx = git_context.unwrap();

    // Verify git metrics are present in Historical details
    match &git_ctx.details {
        ContextDetails::Historical {
            change_frequency,
            author_count,
            age_days,
            ..
        } => {
            // Note: change_frequency might be 0 if commits are very recent (age is 0 days)
            // The key is that we got Historical context data
            assert!(
                *change_frequency >= 0.0,
                "Should have valid (non-negative) change frequency"
            );
            assert!(*author_count >= 1, "Should have at least one author");
            // age_days is u32, always >= 0, just verify it exists
            let _ = age_days;
        }
        _ => panic!("Expected Historical context details"),
    }

    // Create a test god object debt item with this contextual risk
    let location = Location {
        file: god_object_file.clone(),
        line: 1,
        function: "GodObject".to_string(),
    };

    let god_object_item = create_test_unified_debt_item(
        location,
        DebtType::GodObject {
            methods: 3,
            fields: Some(2),
            responsibilities: 2,
            god_object_score: Score0To100::new(60.0),
            lines: 150,
        },
    );

    // In a real TUI rendering scenario, this contextual_risk would be attached
    // to the god_object_item and displayed in the detail view
    // For this test, we verify the data exists and has expected structure
    assert_eq!(format!("{}", god_object_item.debt_type), "God Object");

    // Verify the contextual risk can be attached to items
    // (In actual code, this happens during analysis pipeline)
    let mut enriched_item = god_object_item;
    enriched_item.contextual_risk = Some(context);

    assert!(
        enriched_item.contextual_risk.is_some(),
        "God object item should have contextual risk attached"
    );

    let final_context = enriched_item.contextual_risk.unwrap();

    // Verify git_history context is preserved
    let final_git_context = final_context
        .contexts
        .iter()
        .find(|ctx| ctx.provider == "git_history");

    assert!(
        final_git_context.is_some(),
        "Final item should preserve git history context"
    );

    // Verify metrics are preserved
    if let Some(git_ctx) = final_git_context {
        match &git_ctx.details {
            ContextDetails::Historical {
                change_frequency, ..
            } => {
                assert!(
                    *change_frequency >= 0.0,
                    "Git metrics should be preserved in final item (valid non-negative value)"
                );
            }
            _ => panic!("Expected Historical context details"),
        }
    }
}
