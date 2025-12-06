//! TUI application state management.

use std::time::{Duration, Instant};

/// Visual status of a pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    /// Stage has not started yet
    Pending,
    /// Stage is currently executing
    Active,
    /// Stage has completed successfully
    Completed,
}

/// A sub-task within a pipeline stage
#[derive(Debug, Clone)]
pub struct SubTask {
    /// Name of the sub-task
    pub name: String,
    /// Current status
    pub status: StageStatus,
    /// Progress information (current, total)
    pub progress: Option<(usize, usize)>,
}

/// A pipeline stage in the analysis process
#[derive(Debug, Clone)]
pub struct PipelineStage {
    /// Display name of the stage
    pub name: String,
    /// Current status
    pub status: StageStatus,
    /// Summary metric (e.g., "469 files", "5,432 functions")
    pub metric: Option<String>,
    /// Time taken to complete (if completed)
    pub elapsed: Option<Duration>,
    /// Sub-tasks within this stage
    pub sub_tasks: Vec<SubTask>,
}

impl PipelineStage {
    /// Create a new pending stage
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Pending,
            metric: None,
            elapsed: None,
            sub_tasks: Vec::new(),
        }
    }

    /// Create a stage with sub-tasks
    pub fn with_subtasks(name: impl Into<String>, subtasks: Vec<SubTask>) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Pending,
            metric: None,
            elapsed: None,
            sub_tasks: subtasks,
        }
    }
}

/// Main TUI application state
pub struct App {
    /// All pipeline stages
    pub stages: Vec<PipelineStage>,
    /// Overall progress (0.0 to 1.0)
    pub overall_progress: f64,
    /// Index of currently active stage
    pub current_stage: usize,
    /// Total elapsed time
    pub elapsed_time: Duration,
    /// Start time of analysis
    pub start_time: Instant,

    // Statistics for bottom bar
    /// Total number of functions analyzed
    pub functions_count: usize,
    /// Number of debt items detected
    pub debt_count: usize,
    /// Test coverage percentage
    pub coverage_percent: f64,
    /// Number of threads used
    pub thread_count: usize,

    // Animation state
    /// Current animation frame (0-59 for 60 FPS)
    pub animation_frame: usize,
    /// Last update time
    pub last_update: Instant,
}

impl App {
    /// Create a new application with the standard 9-stage pipeline
    pub fn new() -> Self {
        Self {
            stages: Self::create_default_stages(),
            overall_progress: 0.0,
            current_stage: 0,
            elapsed_time: Duration::from_secs(0),
            start_time: Instant::now(),
            functions_count: 0,
            debt_count: 0,
            coverage_percent: 0.0,
            thread_count: num_cpus::get(),
            animation_frame: 0,
            last_update: Instant::now(),
        }
    }

    /// Create the default 8-stage pipeline structure
    fn create_default_stages() -> Vec<PipelineStage> {
        vec![
            PipelineStage::new("files"),
            PipelineStage::new("parse"),
            PipelineStage::new("call graph"),
            PipelineStage::new("coverage"),
            PipelineStage::with_subtasks(
                "purity analysis",
                vec![
                    SubTask {
                        name: "data flow graph".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "initial detection".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "propagation".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "side effects".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                ],
            ),
            PipelineStage::with_subtasks(
                "context",
                vec![
                    SubTask {
                        name: "critical path".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "dependencies".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "git history".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                ],
            ),
            PipelineStage::with_subtasks(
                "debt scoring",
                vec![
                    SubTask {
                        name: "initialize".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "aggregate debt".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "score functions".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                    SubTask {
                        name: "filter results".to_string(),
                        status: StageStatus::Pending,
                        progress: None,
                    },
                ],
            ),
            PipelineStage::new("prioritization"),
        ]
    }

    /// Update animation state (call at 60 FPS)
    pub fn tick(&mut self) {
        self.elapsed_time = self.start_time.elapsed();
        self.animation_frame = (self.animation_frame + 1) % 60;
        self.last_update = Instant::now();
    }

    /// Start a stage (mark as active)
    pub fn start_stage(&mut self, stage_index: usize) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            stage.status = StageStatus::Active;
            self.current_stage = stage_index;
        }
    }

    /// Complete a stage with a metric summary
    pub fn complete_stage(&mut self, stage_index: usize, metric: impl Into<String>) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            stage.status = StageStatus::Completed;
            stage.metric = Some(metric.into());
            stage.elapsed = Some(self.start_time.elapsed());
        }
    }

    /// Update stage progress metric
    pub fn update_stage_metric(&mut self, stage_index: usize, metric: impl Into<String>) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            stage.metric = Some(metric.into());
        }
    }

    /// Update sub-task status
    pub fn update_subtask(
        &mut self,
        stage_index: usize,
        subtask_index: usize,
        status: StageStatus,
        progress: Option<(usize, usize)>,
    ) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            if let Some(subtask) = stage.sub_tasks.get_mut(subtask_index) {
                subtask.status = status;
                subtask.progress = progress;
            }
        }
    }

    /// Update overall progress (0.0 to 1.0)
    pub fn set_overall_progress(&mut self, progress: f64) {
        self.overall_progress = progress.clamp(0.0, 1.0);
    }

    /// Update statistics
    pub fn update_stats(&mut self, functions: usize, debt: usize, coverage: f64, threads: usize) {
        self.functions_count = functions;
        self.debt_count = debt;
        self.coverage_percent = coverage;
        self.thread_count = threads;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.stages.len(), 8);
        assert_eq!(app.overall_progress, 0.0);
        assert_eq!(app.current_stage, 0);
    }

    #[test]
    fn test_stage_lifecycle() {
        let mut app = App::new();

        // Start first stage
        app.start_stage(0);
        assert_eq!(app.stages[0].status, StageStatus::Active);
        assert_eq!(app.current_stage, 0);

        // Complete first stage
        app.complete_stage(0, "469 files");
        assert_eq!(app.stages[0].status, StageStatus::Completed);
        assert_eq!(app.stages[0].metric, Some("469 files".to_string()));
        assert!(app.stages[0].elapsed.is_some());
    }

    #[test]
    fn test_subtask_updates() {
        let mut app = App::new();

        // Purity analysis stage has subtasks (index 4)
        app.update_subtask(4, 0, StageStatus::Completed, None);
        assert_eq!(app.stages[4].sub_tasks[0].status, StageStatus::Completed);

        app.update_subtask(4, 1, StageStatus::Active, Some((50, 100)));
        assert_eq!(app.stages[4].sub_tasks[1].status, StageStatus::Active);
        assert_eq!(app.stages[4].sub_tasks[1].progress, Some((50, 100)));
    }

    #[test]
    fn test_progress_clamping() {
        let mut app = App::new();

        app.set_overall_progress(0.5);
        assert_eq!(app.overall_progress, 0.5);

        app.set_overall_progress(1.5); // Over 1.0
        assert_eq!(app.overall_progress, 1.0);

        app.set_overall_progress(-0.5); // Under 0.0
        assert_eq!(app.overall_progress, 0.0);
    }

    #[test]
    fn test_animation_tick() {
        let mut app = App::new();
        let initial_frame = app.animation_frame;

        app.tick();
        assert_eq!(app.animation_frame, (initial_frame + 1) % 60);
        assert!(app.elapsed_time.as_nanos() > 0);
    }
}
