---
number: 210
title: Zen Minimalist TUI Progress Visualization with Ratatui
category: optimization
priority: high
status: draft
dependencies: [207, 208, 209]
created: 2025-12-05
---

# Specification 210: Zen Minimalist TUI Progress Visualization with Ratatui

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 207 (Stillwater Effects), Spec 208 (Pure Functions), Spec 209 (Pipeline)

## Context

The current progress reporting in debtmap uses simple progress bars and spinners from the `indicatif` crate:

```
✓ 1/4 Discovering files...469 found - 0s
✓ 2/4 Analyzing complexity...469/469 (100%) - 0s
✓ 3/4 Building call graph...19007/19007 (100%) - 12s
✓ 4/4 Refining analysis... - 0s
/ Computing technical debt priorities
```

**Current Problems**:
- **Hidden work**: Major analysis phases (purity, debt scoring) happen under spinners with no progress
- **No hierarchy**: Sub-tasks invisible, unclear what's actually happening
- **Limited info**: Just percentages, no context about what's being processed
- **Boring**: Functional but not engaging or informative
- **No detail control**: Can't see more/less detail based on terminal size

**Opportunity**: With the new composable pipeline architecture (Spec 209), we have structured stages and sub-tasks. We can create a beautiful, informative TUI that:
- Shows all 9 pipeline stages at once
- Expands active stage to show sub-tasks with animated progress
- Uses clean zen minimalist aesthetic with subtle hierarchy
- Provides rich context without overwhelming
- Adapts to terminal size gracefully

**Inspiration**: The zen minimalist design with subtle hierarchy provides clarity through simplicity, using visual states (✓ done, ▸ active, · pending) and beautiful dotted animations.

## Objective

Implement a next-generation TUI for analysis progress using `ratatui` that:
1. **Visualizes the entire pipeline** - All 9 stages visible with clear status
2. **Shows hierarchical progress** - Active stage expands to reveal sub-tasks
3. **Provides rich context** - Counts, percentages, and current file/function being analyzed
4. **Uses zen minimalist aesthetic** - Clean, spacious, with subtle visual hierarchy
5. **Animates smoothly** - Progress bars fill, arrows slide, numbers count up
6. **Responds to terminal size** - Graceful degradation for small terminals
7. **Integrates with pipeline** - Progress updates driven by pipeline stages (Spec 209)

**Success Criteria**: Users can watch the entire analysis unfold with clear visibility into every stage and sub-task, creating confidence and engagement.

## Requirements

### Functional Requirements

1. **Pipeline Stage Visualization**
   - Display all 9 pipeline stages in a vertical list
   - Three visual states:
     - `✓` Completed (with metric summary)
     - `▸` Active (with sub-task expansion)
     - `·` Pending (dimmed)
   - Show stage-specific metrics (e.g., "469 files", "5,432 functions")

2. **Overall Progress Indicator**
   - Single progress bar at top showing overall completion
   - Format: `▓▓▓▓▓▓▓▓░░░░ 67%`
   - Stage counter: "stage 6/9"
   - Elapsed time: "12.3s"

3. **Active Stage Expansion**
   - Active stage expands to show sub-tasks
   - Sub-tasks indented with dotted leaders
   - Format: `    data flow graph ··················· done`
   - Animated progress for active sub-task: `    propagation ▸▸▸▸▸▸▸▸▸▸▸░░░░`
   - Show progress ratio: "1,234/5,432"

4. **Summary Statistics Bar**
   - Bottom bar with key metrics
   - Format: `functions 5,432  │  debt 127  │  coverage 78.3%  │  threads 8`
   - Updates in real-time as analysis progresses

5. **Smooth Animations**
   - Progress bar fills left-to-right with gradient
   - Active stage `▸` pulses subtly
   - Sub-task arrows `▸▸▸▸` animate sliding right
   - Numbers count up smoothly (not instant jumps)
   - Completed items fade dots to "done"

6. **Terminal Size Adaptation**
   - **Large (>120 cols)**: Full detail with all metrics
   - **Medium (80-120 cols)**: Standard view as designed
   - **Small (<80 cols)**: Collapse to compact mode (no sub-tasks)
   - **Tiny (<40 cols)**: Minimal fallback (simple progress bar)

### Non-Functional Requirements

1. **Performance**
   - TUI updates at 60 FPS (16ms frame budget)
   - No blocking on main thread (async updates)
   - Minimal CPU usage when idle
   - Smooth animation without flickering

2. **Accessibility**
   - Works in terminals without Unicode support (ASCII fallback)
   - Color scheme respects terminal color settings
   - Works with screen readers (semantic structure)
   - Keyboard controls for interaction (if needed)

3. **Compatibility**
   - Works on macOS, Linux, Windows
   - Supports common terminal emulators (iTerm, Terminal.app, Alacritty, etc.)
   - Graceful degradation for limited terminals
   - Falls back to simple progress for non-TTY (CI/pipes)

4. **Integration**
   - Plugs into pipeline progress system (Spec 209)
   - Minimal changes to pipeline stage code
   - Can be disabled with `--no-tui` flag
   - Quiet mode (`DEBTMAP_QUIET`) bypasses TUI entirely

## Acceptance Criteria

- [ ] Ratatui added to `Cargo.toml` and compiles successfully
- [ ] TUI module created at `src/tui/` with core rendering logic
- [ ] Overall progress bar renders with gradient fill (▓░)
- [ ] All 9 pipeline stages displayed with correct visual states (✓ ▸ ·)
- [ ] Active stage expands to show sub-tasks with dotted leaders
- [ ] Sub-task animations work (▸▸▸▸ sliding, dots fading)
- [ ] Summary statistics bar updates in real-time
- [ ] Progress updates driven by pipeline stage events
- [ ] Terminal size detection and responsive layout
- [ ] Smooth 60 FPS rendering with no flicker
- [ ] Graceful fallback for small terminals (<80 cols)
- [ ] ASCII fallback mode for terminals without Unicode
- [ ] Works on macOS, Linux, Windows
- [ ] `--no-tui` flag disables TUI (falls back to simple progress)
- [ ] Non-TTY detection (CI mode) bypasses TUI automatically
- [ ] Example recording (GIF/video) demonstrates the TUI in action
- [ ] Documentation in README shows TUI screenshots

## Technical Details

### Implementation Approach

#### Phase 1: Setup and Dependencies

1. **Add Ratatui Dependency**
   ```toml
   [dependencies]
   ratatui = "0.26"
   crossterm = "0.27"  # For terminal handling
   unicode-width = "0.1"  # For text width calculations
   ```

2. **Create Module Structure**
   ```
   src/tui/
   ├── mod.rs              # Public API
   ├── app.rs              # TUI application state
   ├── renderer.rs         # Rendering logic
   ├── layout.rs           # Layout calculations
   ├── widgets/
   │   ├── mod.rs
   │   ├── pipeline.rs     # Pipeline stage list widget
   │   ├── progress.rs     # Progress bar widget
   │   ├── stats.rs        # Statistics bar widget
   │   └── subtasks.rs     # Sub-task expansion widget
   ├── animation.rs        # Animation helpers
   └── theme.rs            # Color and style definitions
   ```

3. **Terminal Setup**
   ```rust
   // src/tui/mod.rs
   use crossterm::{
       terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
       execute,
   };
   use ratatui::{
       backend::CrosstermBackend,
       Terminal,
   };

   pub struct TuiManager {
       terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
       should_quit: bool,
   }

   impl TuiManager {
       pub fn new() -> Result<Self, std::io::Error> {
           enable_raw_mode()?;
           let mut stdout = std::io::stdout();
           execute!(stdout, EnterAlternateScreen)?;
           let backend = CrosstermBackend::new(stdout);
           let terminal = Terminal::new(backend)?;

           Ok(Self {
               terminal,
               should_quit: false,
           })
       }

       pub fn render(&mut self, app: &App) -> Result<(), std::io::Error> {
           self.terminal.draw(|f| render_ui(f, app))?;
           Ok(())
       }
   }

   impl Drop for TuiManager {
       fn drop(&mut self) {
           let _ = disable_raw_mode();
           let _ = execute!(
               self.terminal.backend_mut(),
               LeaveAlternateScreen
           );
       }
   }
   ```

#### Phase 2: Application State

```rust
// src/tui/app.rs

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StageStatus {
    Pending,
    Active,
    Completed,
}

#[derive(Debug, Clone)]
pub struct SubTask {
    pub name: String,
    pub status: StageStatus,
    pub progress: Option<(usize, usize)>,  // (current, total)
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub name: String,
    pub status: StageStatus,
    pub metric: Option<String>,  // e.g., "469 files", "5,432 functions"
    pub elapsed: Option<Duration>,
    pub sub_tasks: Vec<SubTask>,
}

pub struct App {
    pub stages: Vec<PipelineStage>,
    pub overall_progress: f64,  // 0.0 to 1.0
    pub current_stage: usize,
    pub elapsed_time: Duration,
    pub start_time: Instant,

    // Statistics
    pub functions_count: usize,
    pub debt_count: usize,
    pub coverage_percent: f64,
    pub thread_count: usize,

    // Animation state
    pub animation_frame: usize,
    pub last_update: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            stages: vec![
                PipelineStage {
                    name: "files".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "parse".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "call graph".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "trait resolution".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "coverage".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "purity analysis".to_string(),
                    status: StageStatus::Active,
                    metric: Some("1,234/5,432".to_string()),
                    elapsed: None,
                    sub_tasks: vec![
                        SubTask {
                            name: "data flow graph".to_string(),
                            status: StageStatus::Completed,
                            progress: None,
                        },
                        SubTask {
                            name: "initial detection".to_string(),
                            status: StageStatus::Completed,
                            progress: None,
                        },
                        SubTask {
                            name: "propagation".to_string(),
                            status: StageStatus::Active,
                            progress: Some((234, 987)),
                        },
                        SubTask {
                            name: "side effects".to_string(),
                            status: StageStatus::Pending,
                            progress: None,
                        },
                    ],
                },
                PipelineStage {
                    name: "context".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "debt scoring".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
                PipelineStage {
                    name: "prioritization".to_string(),
                    status: StageStatus::Pending,
                    metric: None,
                    elapsed: None,
                    sub_tasks: vec![],
                },
            ],
            overall_progress: 0.67,
            current_stage: 5,
            elapsed_time: Duration::from_secs(12),
            start_time: Instant::now(),
            functions_count: 5432,
            debt_count: 127,
            coverage_percent: 78.3,
            thread_count: 8,
            animation_frame: 0,
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        self.elapsed_time = self.start_time.elapsed();
        self.animation_frame = (self.animation_frame + 1) % 60;
        self.last_update = Instant::now();
    }

    pub fn update_stage(&mut self, stage_index: usize, status: StageStatus, metric: Option<String>) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            stage.status = status;
            stage.metric = metric;
        }
    }

    pub fn update_sub_task(&mut self, stage_index: usize, sub_task_index: usize, progress: Option<(usize, usize)>) {
        if let Some(stage) = self.stages.get_mut(stage_index) {
            if let Some(sub_task) = stage.sub_tasks.get_mut(sub_task_index) {
                sub_task.progress = progress;
            }
        }
    }
}
```

#### Phase 3: Rendering Logic

```rust
// src/tui/renderer.rs

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Gauge},
    Frame,
};
use crate::tui::app::{App, StageStatus};

pub fn render_ui(frame: &mut Frame, app: &App) {
    let size = frame.size();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(5),   // Header (title + progress bar)
            Constraint::Min(10),     // Main content (pipeline stages)
            Constraint::Length(3),   // Footer (statistics)
        ])
        .split(size);

    // Render header
    render_header(frame, app, chunks[0]);

    // Render pipeline stages
    render_pipeline(frame, app, chunks[1]);

    // Render footer
    render_footer(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Title line
            Constraint::Length(1),   // Empty
            Constraint::Length(1),   // Progress bar
            Constraint::Length(1),   // Stage counter
        ])
        .split(area);

    // Title
    let title = Line::from(vec![
        Span::raw("debtmap"),
        Span::raw("                                                   "),
        Span::styled(
            format!("{}s", app.elapsed_time.as_secs_f64()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(title), header_chunks[0]);

    // Progress bar with gradient
    let progress_bar = render_progress_bar(app.overall_progress, area.width as usize);
    frame.render_widget(
        Paragraph::new(progress_bar),
        header_chunks[2],
    );

    // Stage counter
    let stage_info = format!("stage {}/9", app.current_stage + 1);
    frame.render_widget(
        Paragraph::new(stage_info).style(Style::default().fg(Color::DarkGray)),
        header_chunks[3],
    );
}

fn render_progress_bar(progress: f64, width: usize) -> Line<'static> {
    let filled = (progress * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    let bar = format!(
        "{}{}",
        "▓".repeat(filled),
        "░".repeat(empty)
    );

    let percentage = format!(" {:.0}%", progress * 100.0);

    Line::from(vec![
        Span::styled(bar, Style::default().fg(Color::Cyan)),
        Span::styled(percentage, Style::default().fg(Color::White)),
    ])
}

fn render_pipeline(frame: &mut Frame, app: &App, area: Rect) {
    let mut text = Text::default();

    for (i, stage) in app.stages.iter().enumerate() {
        // Add spacing
        text.extend(Text::raw("\n"));

        // Stage line
        let (icon, style) = match stage.status {
            StageStatus::Completed => ("✓", Style::default().fg(Color::Green)),
            StageStatus::Active => ("▸", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            StageStatus::Pending => ("·", Style::default().fg(Color::DarkGray)),
        };

        let stage_line = create_stage_line(icon, &stage.name, stage.metric.as_deref(), style, area.width as usize);
        text.extend(Text::from(stage_line));

        // Sub-tasks (only for active stage)
        if stage.status == StageStatus::Active && !stage.sub_tasks.is_empty() {
            text.extend(Text::raw("\n"));

            for sub_task in &stage.sub_tasks {
                let sub_line = create_subtask_line(sub_task, app.animation_frame, area.width as usize);
                text.extend(Text::from(sub_line));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(text),
        area,
    );
}

fn create_stage_line(icon: &str, name: &str, metric: Option<&str>, style: Style, width: usize) -> Line<'static> {
    let metric_str = metric.unwrap_or("");
    let metric_len = metric_str.len();

    // Calculate spacing
    let icon_and_name = format!("{}  {}", icon, name);
    let available = width.saturating_sub(icon_and_name.len() + metric_len + 2);

    Line::from(vec![
        Span::styled(icon, style),
        Span::raw("  "),
        Span::styled(name.to_string(), style),
        Span::raw(" ".repeat(available)),
        Span::styled(metric_str.to_string(), Style::default().fg(Color::DarkGray)),
    ])
}

fn create_subtask_line(sub_task: &crate::tui::app::SubTask, frame: usize, width: usize) -> Line<'static> {
    let indent = "    ";
    let name_with_indent = format!("{}{}", indent, sub_task.name);

    match sub_task.status {
        StageStatus::Completed => {
            // Dotted leader to "done"
            let dots_needed = width.saturating_sub(name_with_indent.len() + 5);
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::styled(
                    "·".repeat(dots_needed),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(" done", Style::default().fg(Color::Green)),
            ])
        }
        StageStatus::Active => {
            // Animated arrows
            if let Some((current, total)) = sub_task.progress {
                let progress = current as f64 / total as f64;
                let arrow_count = ((progress * 30.0) as usize).min(30);
                let dots_count = 30_usize.saturating_sub(arrow_count);

                // Animate arrows sliding
                let arrow_offset = (frame / 3) % 3;
                let arrows = match arrow_offset {
                    0 => "▸",
                    1 => "▹",
                    _ => "▸",
                };

                Line::from(vec![
                    Span::raw(name_with_indent),
                    Span::raw(" "),
                    Span::styled(
                        arrows.repeat(arrow_count),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        "·".repeat(dots_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            } else {
                Line::from(Span::raw(name_with_indent))
            }
        }
        StageStatus::Pending => {
            // Just dots
            let dots_needed = width.saturating_sub(name_with_indent.len());
            Line::from(vec![
                Span::raw(name_with_indent),
                Span::styled(
                    "·".repeat(dots_needed),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let stats = format!(
        "functions {}  │  debt {}  │  coverage {:.1}%  │  threads {}",
        app.functions_count,
        app.debt_count,
        app.coverage_percent,
        app.thread_count
    );

    frame.render_widget(
        Paragraph::new(stats).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
```

#### Phase 4: Integration with Pipeline

```rust
// src/pipeline/progress.rs

use crate::tui::{TuiManager, App};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct PipelineProgressReporter {
    tui: Arc<Mutex<Option<TuiManager>>>,
    app: Arc<Mutex<App>>,
    update_thread: Option<thread::JoinHandle<()>>,
}

impl PipelineProgressReporter {
    pub fn new() -> Result<Self, std::io::Error> {
        let tui = TuiManager::new()?;
        let app = App::new();

        let tui_arc = Arc::new(Mutex::new(Some(tui)));
        let app_arc = Arc::new(Mutex::new(app));

        // Spawn render thread (60 FPS)
        let tui_clone = Arc::clone(&tui_arc);
        let app_clone = Arc::clone(&app_arc);
        let update_thread = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(16)); // ~60 FPS

                let mut app = app_clone.lock().unwrap();
                app.update();

                if let Some(ref mut tui) = *tui_clone.lock().unwrap() {
                    let _ = tui.render(&app);
                }
            }
        });

        Ok(Self {
            tui: tui_arc,
            app: app_arc,
            update_thread: Some(update_thread),
        })
    }

    pub fn start_stage(&self, stage_index: usize, name: &str) {
        let mut app = self.app.lock().unwrap();
        app.update_stage(stage_index, crate::tui::app::StageStatus::Active, None);
        app.current_stage = stage_index;
    }

    pub fn complete_stage(&self, stage_index: usize, metric: &str) {
        let mut app = self.app.lock().unwrap();
        app.update_stage(
            stage_index,
            crate::tui::app::StageStatus::Completed,
            Some(metric.to_string()),
        );
    }

    pub fn update_progress(&self, stage_index: usize, current: usize, total: usize) {
        let mut app = self.app.lock().unwrap();
        let metric = format!("{}/{}", current, total);
        app.update_stage(stage_index, crate::tui::app::StageStatus::Active, Some(metric));
    }
}
```

#### Phase 5: Terminal Size Adaptation

```rust
// src/tui/layout.rs

pub enum LayoutMode {
    Full,      // >120 cols: Full detail
    Standard,  // 80-120 cols: Standard view
    Compact,   // 40-80 cols: No sub-tasks
    Minimal,   // <40 cols: Just progress bar
}

impl LayoutMode {
    pub fn from_terminal_width(width: u16) -> Self {
        match width {
            0..=39 => Self::Minimal,
            40..=79 => Self::Compact,
            80..=119 => Self::Standard,
            _ => Self::Full,
        }
    }
}

pub fn render_ui_adaptive(frame: &mut Frame, app: &App) {
    let mode = LayoutMode::from_terminal_width(frame.size().width);

    match mode {
        LayoutMode::Full | LayoutMode::Standard => render_ui(frame, app),
        LayoutMode::Compact => render_compact(frame, app),
        LayoutMode::Minimal => render_minimal(frame, app),
    }
}

fn render_compact(frame: &mut Frame, app: &App) {
    // Omit sub-tasks, just show stages
    // ... simplified rendering
}

fn render_minimal(frame: &mut Frame, app: &App) {
    // Just show progress bar and current stage
    // ... minimal rendering
}
```

#### Phase 6: Animation System

```rust
// src/tui/animation.rs

use std::time::Duration;

pub struct AnimationController {
    frame: usize,
    fps: usize,
    frame_duration: Duration,
}

impl AnimationController {
    pub fn new(fps: usize) -> Self {
        Self {
            frame: 0,
            fps,
            frame_duration: Duration::from_millis(1000 / fps as u64),
        }
    }

    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % (self.fps * 10); // Loop every 10 seconds
    }

    pub fn get_arrow_char(&self) -> &'static str {
        // Cycle through arrow variants for animation
        match (self.frame / 3) % 3 {
            0 => "▸",
            1 => "▹",
            _ => "▸",
        }
    }

    pub fn get_spinner_char(&self) -> &'static str {
        // Classic spinner
        match (self.frame / 8) % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        }
    }

    pub fn get_pulse_alpha(&self) -> f32 {
        // Sine wave for pulsing effect
        let phase = self.frame as f32 / self.fps as f32;
        (phase * std::f32::consts::PI * 2.0).sin() * 0.3 + 0.7
    }
}
```

### Architecture Changes

**New Components**:
```
src/tui/                    # New TUI module
  ├── mod.rs
  ├── app.rs                # Application state
  ├── renderer.rs           # Rendering logic
  ├── layout.rs             # Responsive layout
  ├── animation.rs          # Animation helpers
  ├── theme.rs              # Color schemes
  └── widgets/
      ├── pipeline.rs
      ├── progress.rs
      ├── stats.rs
      └── subtasks.rs
```

**Modified Components**:
- `src/commands/analyze.rs` - Integrate TUI manager
- `src/pipeline/mod.rs` - Add progress reporting hooks
- `src/progress/mod.rs` - Detect TTY and choose renderer

### APIs and Interfaces

```rust
pub mod tui {
    // Core TUI manager
    pub struct TuiManager { ... }

    impl TuiManager {
        pub fn new() -> Result<Self, std::io::Error>;
        pub fn render(&mut self, app: &App) -> Result<(), std::io::Error>;
    }

    // Application state
    pub struct App { ... }

    impl App {
        pub fn new() -> Self;
        pub fn update(&mut self);
        pub fn update_stage(&mut self, ...);
        pub fn update_sub_task(&mut self, ...);
    }

    // Integration with pipeline
    pub struct PipelineProgressReporter { ... }

    impl PipelineProgressReporter {
        pub fn new() -> Result<Self, std::io::Error>;
        pub fn start_stage(&self, stage_index: usize, name: &str);
        pub fn complete_stage(&self, stage_index: usize, metric: &str);
        pub fn update_progress(&self, stage_index: usize, current: usize, total: usize);
    }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 207 (Stillwater Effects Integration) - Optional, improves integration
  - Spec 208 (Pure Function Extraction) - Optional, cleaner code
  - Spec 209 (Composable Pipeline Architecture) - Required for structured progress
- **Affected Components**:
  - `src/commands/analyze.rs` - Add TUI initialization
  - `src/pipeline/mod.rs` - Hook progress reporting
  - `src/progress/mod.rs` - Add TUI mode selection
- **External Dependencies**:
  - `ratatui = "0.26"` - TUI framework
  - `crossterm = "0.27"` - Terminal handling
  - `unicode-width = "0.1"` - Text width calculations

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_updates() {
        let mut app = App::new();
        app.update_stage(0, StageStatus::Completed, Some("469".to_string()));
        assert_eq!(app.stages[0].status, StageStatus::Completed);
        assert_eq!(app.stages[0].metric, Some("469".to_string()));
    }

    #[test]
    fn test_progress_bar_rendering() {
        let line = render_progress_bar(0.67, 50);
        // Verify gradient characters
        assert!(line.to_string().contains("▓"));
        assert!(line.to_string().contains("░"));
    }

    #[test]
    fn test_layout_mode_selection() {
        assert_eq!(LayoutMode::from_terminal_width(30), LayoutMode::Minimal);
        assert_eq!(LayoutMode::from_terminal_width(80), LayoutMode::Standard);
        assert_eq!(LayoutMode::from_terminal_width(150), LayoutMode::Full);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_pipeline_with_tui() {
    // Create TUI reporter
    let reporter = PipelineProgressReporter::new().unwrap();

    // Simulate pipeline progression
    reporter.start_stage(0, "files");
    std::thread::sleep(std::time::Duration::from_millis(100));
    reporter.complete_stage(0, "469");

    reporter.start_stage(1, "parse");
    reporter.update_progress(1, 234, 469);
    std::thread::sleep(std::time::Duration::from_millis(100));
    reporter.complete_stage(1, "5,432");

    // Verify state updated correctly
    let app = reporter.app.lock().unwrap();
    assert_eq!(app.stages[0].status, StageStatus::Completed);
    assert_eq!(app.stages[1].status, StageStatus::Completed);
}
```

### Manual Testing

- [ ] Test on iTerm2 (macOS)
- [ ] Test on Terminal.app (macOS)
- [ ] Test on Alacritty
- [ ] Test on Windows Terminal
- [ ] Test on Linux terminal (GNOME Terminal)
- [ ] Test with different terminal sizes (resize during analysis)
- [ ] Test with Unicode disabled (ASCII fallback)
- [ ] Test in non-TTY mode (pipe to file)
- [ ] Test with `--no-tui` flag
- [ ] Test with `DEBTMAP_QUIET=1`
- [ ] Record GIF/video of analysis in action

### Performance Tests

```rust
#[bench]
fn bench_tui_rendering(b: &mut Bencher) {
    let app = App::new();
    let mut backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    b.iter(|| {
        terminal.draw(|f| render_ui(f, &app)).unwrap();
    });
}

#[test]
fn test_60fps_target() {
    let start = std::time::Instant::now();
    let app = App::new();

    // Render 60 frames
    for _ in 0..60 {
        // ... render logic
    }

    let elapsed = start.elapsed();
    assert!(elapsed.as_secs_f64() < 1.0); // Should complete in <1 second
}
```

## Documentation Requirements

### User Documentation

Update `README.md`:
```markdown
## Analysis Progress

Debtmap features a beautiful zen minimalist TUI for watching analysis:

![Debtmap TUI](docs/tui-demo.gif)

### Features

- **Full pipeline visibility**: See all 9 analysis stages at once
- **Hierarchical progress**: Active stage expands to show sub-tasks
- **Rich context**: Counts, percentages, and real-time statistics
- **Smooth animations**: Progress bars fill, arrows slide, numbers count up
- **Responsive**: Adapts to your terminal size

### Options

- `--no-tui`: Disable TUI (simple progress bars)
- `DEBTMAP_QUIET=1`: No progress output (for CI)
```

### Developer Documentation

Add `docs/TUI_ARCHITECTURE.md`:
```markdown
# TUI Architecture

## Overview

The TUI system uses `ratatui` for rendering and `crossterm` for terminal
handling. Progress updates flow from the pipeline through a progress
reporter to the TUI application state.

## Data Flow

```
Pipeline Stage
  ↓
PipelineProgressReporter
  ↓
App State (shared via Arc<Mutex>)
  ↓
Render Thread (60 FPS)
  ↓
TUI Display
```

## Adding New Stages

To add a new pipeline stage to the TUI:

1. Update `App::new()` to include the stage
2. Add progress hooks in pipeline stage code
3. Call `reporter.start_stage()` and `reporter.complete_stage()`
```

## Implementation Notes

### Best Practices

1. **Thread Safety**
   - Use `Arc<Mutex<>>` for shared state
   - Keep lock durations minimal
   - Render on separate thread to avoid blocking

2. **Performance**
   - Target 60 FPS (16ms frame budget)
   - Minimize allocations in render loop
   - Cache computed layouts where possible

3. **Terminal Compatibility**
   - Always provide ASCII fallback
   - Detect TTY vs pipe
   - Handle terminal resize events

4. **Animation Timing**
   - Use frame counters, not wall clock
   - Keep animations subtle and non-distracting
   - Provide way to disable animations

### Common Pitfalls

1. **Blocking the Main Thread**
   ```rust
   // Bad: Render on main thread
   pipeline.execute().and_then(|_| tui.render());

   // Good: Render on separate thread
   let render_thread = spawn_render_loop(tui, app);
   pipeline.execute();
   ```

2. **Holding Locks Too Long**
   ```rust
   // Bad: Hold lock during render
   let app = app_mutex.lock().unwrap();
   terminal.draw(|f| render_ui(f, &app))?;
   // Lock held for entire render!

   // Good: Clone what you need
   let app_snapshot = app_mutex.lock().unwrap().clone();
   terminal.draw(|f| render_ui(f, &app_snapshot))?;
   ```

3. **Unicode Assumptions**
   ```rust
   // Bad: Assume Unicode works
   let arrow = "▸";

   // Good: Check terminal capabilities
   let arrow = if supports_unicode() { "▸" } else { ">" };
   ```

## Migration and Compatibility

### Graceful Degradation

1. **Non-TTY Detection**
   ```rust
   if atty::is(atty::Stream::Stdout) {
       // Use TUI
       let tui = TuiManager::new()?;
   } else {
       // Use simple progress bars
       let progress = SimpleProgress::new();
   }
   ```

2. **Feature Flag**
   - `--no-tui` flag explicitly disables TUI
   - Falls back to existing `indicatif` progress bars
   - Ensures CI/automation works unchanged

3. **Environment Variable**
   - `DEBTMAP_QUIET=1` bypasses all progress (including TUI)
   - Existing behavior preserved

### Backward Compatibility

- Old progress system remains functional
- TUI is opt-in enhancement, not replacement
- CLI flags and environment variables unchanged

## Success Metrics

- [ ] TUI renders at stable 60 FPS during analysis
- [ ] CPU usage < 5% for TUI rendering thread
- [ ] Works on macOS, Linux, Windows
- [ ] Graceful fallback for terminals without Unicode
- [ ] User feedback positive (GitHub issues/discussions)
- [ ] Demo GIF included in README generates interest

## Future Enhancements

Potential future improvements (not in scope for this spec):

- **Keyboard controls**: Pause/resume, expand/collapse stages
- **Color themes**: Light/dark mode, customizable colors
- **Export**: Save TUI output as HTML or SVG
- **Live graphs**: CPU/memory usage over time
- **Interactive mode**: Click to expand stages, view details
- **Multiple pipeline views**: Split screen for parallel analysis

## References

- [Ratatui Documentation](https://ratatui.rs/)
- [Crossterm Documentation](https://docs.rs/crossterm/)
- [TUI Best Practices](https://github.com/ratatui-org/ratatui/blob/main/CONTRIBUTING.md)
- [Spec 209: Composable Pipeline Architecture](./209-composable-pipeline-architecture.md)
