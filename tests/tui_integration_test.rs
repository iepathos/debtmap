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

    // Verify all 8 stages are created
    assert_eq!(app.stages.len(), 8);

    // Verify stage names
    assert_eq!(app.stages[0].name, "files");
    assert_eq!(app.stages[1].name, "parse");
    assert_eq!(app.stages[2].name, "call graph");
    assert_eq!(app.stages[3].name, "coverage");
    assert_eq!(app.stages[4].name, "purity analysis");
    assert_eq!(app.stages[5].name, "context");
    assert_eq!(app.stages[6].name, "debt scoring");
    assert_eq!(app.stages[7].name, "prioritization");

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

    // Purity analysis stage (index 4) has subtasks
    assert!(!app.stages[4].sub_tasks.is_empty());

    // Update first subtask
    app.update_subtask(4, 0, StageStatus::Completed, None);
    assert_eq!(app.stages[4].sub_tasks[0].status, StageStatus::Completed);

    // Update second subtask with progress
    app.update_subtask(4, 1, StageStatus::Active, Some((50, 100)));
    assert_eq!(app.stages[4].sub_tasks[1].status, StageStatus::Active);
    assert_eq!(app.stages[4].sub_tasks[1].progress, Some((50, 100)));
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
    app.update_stats(1234, 56, 78.5, 8);
    assert_eq!(app.functions_count, 1234);
    assert_eq!(app.debt_count, 56);
    assert_eq!(app.coverage_percent, 78.5);
    assert_eq!(app.thread_count, 8);
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

    // Stage 1: File discovery
    app.start_stage(0);
    app.set_overall_progress(0.0);
    app.tick();
    app.complete_stage(0, "469 files");
    app.set_overall_progress(0.11);

    // Stage 2: Parsing
    app.start_stage(1);
    app.tick();
    app.complete_stage(1, "469 files parsed");
    app.set_overall_progress(0.22);

    // Stage 3: Call graph
    app.start_stage(2);
    app.tick();
    app.complete_stage(2, "5432 functions");
    app.set_overall_progress(0.33);

    // Stage 4: Coverage
    app.start_stage(3);
    app.tick();
    app.complete_stage(3, "loaded");
    app.set_overall_progress(0.55);

    // Stage 5: Purity analysis
    app.start_stage(4);

    // Update subtasks
    app.update_subtask(4, 0, StageStatus::Completed, None);
    app.tick();
    app.update_subtask(4, 1, StageStatus::Active, Some((50, 100)));
    app.tick();
    app.update_subtask(4, 1, StageStatus::Completed, None);
    app.tick();

    app.complete_stage(4, "5432 functions analyzed");
    app.set_overall_progress(0.66);

    // Stage 6: Context
    app.start_stage(5);
    app.tick();
    app.complete_stage(5, "loaded");
    app.set_overall_progress(0.77);

    // Stage 7: Debt scoring
    app.start_stage(6);
    app.tick();
    app.complete_stage(6, "123 items scored");
    app.set_overall_progress(0.88);

    // Stage 8: Prioritization
    app.start_stage(7);
    app.tick();
    app.complete_stage(7, "complete");
    app.set_overall_progress(1.0);

    // Verify final state
    assert_eq!(app.overall_progress, 1.0);
    assert_eq!(app.current_stage, 7);

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

    // Verify purity analysis stage (index 4) has the correct subtasks
    let purity_stage = &app.stages[4];
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

    // Verify context stage (index 6) has the correct subtasks
    let context_stage = &app.stages[5];
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
    assert_eq!(app.stages[5].sub_tasks.len(), 3);

    // Simulate context stage being skipped (enable_context=false)
    // When context is disabled, the stage starts and completes immediately
    // with "skipped" metric, and subsections are not updated
    app.start_stage(5);
    assert_eq!(app.stages[5].status, StageStatus::Active);

    // Complete the context stage as "skipped" without updating subsections
    app.complete_stage(5, "skipped");
    assert_eq!(app.stages[5].status, StageStatus::Completed);
    assert_eq!(app.stages[5].metric, Some("skipped".to_string()));

    // All subsections should remain in Pending state when context is disabled
    for subtask in &app.stages[5].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
        assert!(subtask.progress.is_none());
    }
}

#[test]
fn test_context_subsection_lifecycle() {
    let mut app = App::new();

    // Start the context stage
    app.start_stage(5);
    assert_eq!(app.stages[5].status, StageStatus::Active);

    // All subsections should start as Pending
    for subtask in &app.stages[5].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Pending);
    }

    // Subsection 0: Critical path analysis - Pending → Active → Completed
    app.update_subtask(5, 0, StageStatus::Active, None);
    assert_eq!(app.stages[5].sub_tasks[0].status, StageStatus::Active);
    assert_eq!(app.stages[5].sub_tasks[0].name, "critical path");
    app.tick();

    app.update_subtask(5, 0, StageStatus::Completed, None);
    assert_eq!(app.stages[5].sub_tasks[0].status, StageStatus::Completed);

    // Subsection 1: Dependencies - Pending → Active → Completed
    app.update_subtask(5, 1, StageStatus::Active, None);
    assert_eq!(app.stages[5].sub_tasks[1].status, StageStatus::Active);
    assert_eq!(app.stages[5].sub_tasks[1].name, "dependencies");
    app.tick();

    app.update_subtask(5, 1, StageStatus::Completed, None);
    assert_eq!(app.stages[5].sub_tasks[1].status, StageStatus::Completed);

    // Subsection 2: Git history - Pending → Active → Completed
    app.update_subtask(5, 2, StageStatus::Active, None);
    assert_eq!(app.stages[5].sub_tasks[2].status, StageStatus::Active);
    assert_eq!(app.stages[5].sub_tasks[2].name, "git history");
    app.tick();

    app.update_subtask(5, 2, StageStatus::Completed, None);
    assert_eq!(app.stages[5].sub_tasks[2].status, StageStatus::Completed);

    // Complete the overall context stage
    app.complete_stage(5, "loaded");
    assert_eq!(app.stages[5].status, StageStatus::Completed);
    assert_eq!(app.stages[5].metric, Some("loaded".to_string()));

    // All subsections should be completed
    for subtask in &app.stages[5].sub_tasks {
        assert_eq!(subtask.status, StageStatus::Completed);
    }
}
