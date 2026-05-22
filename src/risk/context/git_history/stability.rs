//! Pure functions for git-history stability classification and messaging.

/// Stability classification derived from change frequency, bug density, and age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityStatus {
    HighlyUnstable,
    FrequentlyChanged,
    BugProne,
    MatureStable,
    RelativelyStable,
}

/// Calculate bug density as a ratio of bug fixes to total commits.
pub fn calculate_bug_density(bug_fix_count: usize, total_commits: usize) -> f64 {
    if total_commits > 0 {
        bug_fix_count as f64 / total_commits as f64
    } else {
        0.0
    }
}

/// Classify risk contribution from change frequency and bug density (capped at 2.0).
pub fn classify_risk_contribution(change_frequency: f64, bug_density: f64) -> f64 {
    let bug_contribution = bug_density * 1.5;
    let freq_contribution = (change_frequency / 20.0).min(0.5);
    (bug_contribution + freq_contribution).min(2.0)
}

/// Determine stability status from historical metrics.
pub fn determine_stability_status(
    change_frequency: f64,
    bug_density: f64,
    age_days: u64,
) -> StabilityStatus {
    match (change_frequency, bug_density, age_days) {
        (freq, bug, _) if freq > 5.0 && bug > 0.3 => StabilityStatus::HighlyUnstable,
        (freq, _, _) if freq > 2.0 => StabilityStatus::FrequentlyChanged,
        (_, bug, _) if bug > 0.2 => StabilityStatus::BugProne,
        (_, _, age) if age > 365 => StabilityStatus::MatureStable,
        _ => StabilityStatus::RelativelyStable,
    }
}

/// Format a human-readable stability message for the given status.
pub fn format_stability_message(
    status: StabilityStatus,
    change_frequency: f64,
    bug_density: f64,
    age_days: u64,
    author_count: usize,
) -> String {
    match status {
        StabilityStatus::HighlyUnstable => format!(
            "Highly unstable: {:.1} changes/month, {:.0}% bug fixes",
            change_frequency,
            bug_density * 100.0
        ),
        StabilityStatus::FrequentlyChanged => format!(
            "Frequently changed: {change_frequency:.1} changes/month by {author_count} authors"
        ),
        StabilityStatus::BugProne => format!(
            "Bug-prone: {:.0}% of commits are bug fixes",
            bug_density * 100.0
        ),
        StabilityStatus::MatureStable => format!("Mature and stable: {age_days} days old"),
        StabilityStatus::RelativelyStable => {
            format!("Relatively stable: {change_frequency:.1} changes/month")
        }
    }
}

/// Build a full historical context explanation from metrics.
pub fn explain_historical_context(
    change_frequency: f64,
    bug_density: f64,
    age_days: u64,
    author_count: usize,
) -> String {
    let status = determine_stability_status(change_frequency, bug_density, age_days);
    format_stability_message(
        status,
        change_frequency,
        bug_density,
        age_days,
        author_count,
    )
}
