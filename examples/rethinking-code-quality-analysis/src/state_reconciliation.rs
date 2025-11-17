use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Online,
    Offline,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct State {
    pub mode: Mode,
    active_connections: usize,
    pending_writes: usize,
    allow_reconnect: bool,
    config_version: u32,
}

impl State {
    pub fn has_active_connections(&self) -> bool {
        self.active_connections > 0
    }

    pub fn has_pending_writes(&self) -> bool {
        self.pending_writes > 0
    }

    pub fn allows_reconnect(&self) -> bool {
        self.allow_reconnect
    }
}

#[derive(Debug)]
pub enum Action {
    DrainConnections,
    FlushWrites,
    EstablishConnections,
    ScheduleRestart,
}

#[derive(Debug)]
pub struct ConfigDiff {
    requires_restart: bool,
}

impl ConfigDiff {
    pub fn requires_restart(&self) -> bool {
        self.requires_restart
    }
}

pub fn drain_connections() -> Action {
    Action::DrainConnections
}

pub fn flush_writes() -> Action {
    Action::FlushWrites
}

pub fn establish_connections() -> Action {
    Action::EstablishConnections
}

pub fn schedule_restart() -> Action {
    Action::ScheduleRestart
}

pub fn calculate_config_diff(current: &State, target: &State) -> Option<ConfigDiff> {
    if current.config_version != target.config_version {
        Some(ConfigDiff {
            requires_restart: true,
        })
    } else {
        None
    }
}

/// Function B: Moderate Cyclomatic Complexity, High Risk
/// This function has lower cyclomatic complexity (9) but higher cognitive complexity
/// due to nested conditionals with complex interdependencies and state transitions.
pub fn reconcile_state(current: State, target: State) -> Result<Vec<Action>> {
    let mut actions = vec![];

    if current.mode != target.mode {
        if current.has_active_connections() {
            if target.mode == Mode::Offline {
                actions.push(drain_connections());
                if current.has_pending_writes() {
                    actions.push(flush_writes());
                }
            }
        } else if target.allows_reconnect() {
            actions.push(establish_connections());
        }
    }

    if let Some(diff) = calculate_config_diff(&current, &target) {
        if diff.requires_restart() {
            actions.push(schedule_restart());
        }
    }

    Ok(actions)
}
