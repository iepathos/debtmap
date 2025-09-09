use crate::core::Priority;

/// Get health status emoji based on score
pub fn get_health_emoji(score: u32) -> &'static str {
    match score {
        90..=100 => "💚",
        70..=89 => "💛",
        50..=69 => "🟠",
        _ => "🔴",
    }
}

/// Get complexity status indicator
pub fn get_complexity_status(avg: f64) -> &'static str {
    match avg {
        x if x <= 5.0 => "Excellent",
        x if x <= 10.0 => "Good",
        x if x <= 15.0 => "Moderate",
        _ => "Needs Attention",
    }
}

/// Get coverage status indicator
pub fn get_coverage_status(coverage: f64) -> &'static str {
    match coverage {
        x if x >= 80.0 => "Excellent",
        x if x >= 60.0 => "Good",
        x if x >= 40.0 => "Fair",
        _ => "Poor",
    }
}

/// Get debt status based on count
pub fn get_debt_status(count: usize) -> &'static str {
    match count {
        0..=5 => "Minimal",
        6..=15 => "Moderate",
        16..=30 => "Significant",
        _ => "High",
    }
}

/// Get trend indicator for changes
pub fn get_trend_indicator(_change: f64) -> &'static str {
    "➡️" // Placeholder for future trend analysis
}

/// Get complexity indicator with emoji
pub fn get_complexity_indicator(complexity: f64) -> &'static str {
    match complexity {
        x if x <= 5.0 => "🟢 Low",
        x if x <= 10.0 => "🟡 Med",
        x if x <= 20.0 => "🟠 High",
        _ => "🔴 Critical",
    }
}

/// Get coverage indicator with emoji
pub fn get_coverage_indicator(coverage: f64) -> &'static str {
    match coverage {
        x if x >= 0.8 => "🟢 High",
        x if x >= 0.5 => "🟡 Med",
        x if x >= 0.2 => "🟠 Low",
        _ => "🔴 None",
    }
}

/// Get risk indicator with emoji
pub fn get_risk_indicator(risk: f64) -> &'static str {
    match risk {
        x if x <= 3.0 => "🟢 Low",
        x if x <= 6.0 => "🟡 Medium",
        x if x <= 8.0 => "🟠 High",
        _ => "🔴 Critical",
    }
}

/// Get priority label for items
pub fn get_priority_label(index: usize) -> &'static str {
    match index {
        0 => "🔴 Critical",
        1 => "🟠 High",
        2 => "🟡 Medium",
        _ => "🟢 Low",
    }
}

/// Calculate category severity based on debt items priority
pub fn calculate_category_severity(priorities: &[Priority]) -> &'static str {
    let max_priority = priorities.iter().max().unwrap_or(&Priority::Low);

    match max_priority {
        Priority::Critical => "🔴 Critical",
        Priority::High => "🟠 High",
        Priority::Medium => "🟡 Medium",
        Priority::Low => "🟢 Low",
    }
}

/// Create a simple text-based sparkline
pub fn create_sparkline(values: &[u32]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let max = *values.iter().max().unwrap_or(&1);
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    values
        .iter()
        .map(|&v| {
            let index = if max == 0 {
                0
            } else {
                ((v as f64 / max as f64) * 7.0) as usize
            };
            chars[index.min(7)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_creation() {
        let values = vec![1, 3, 2, 5, 4];
        let sparkline = create_sparkline(&values);
        assert_eq!(sparkline.chars().count(), 5);
    }

    #[test]
    fn test_health_score_indicators() {
        assert_eq!(get_health_emoji(95), "💚");
        assert_eq!(get_health_emoji(75), "💛");
        assert_eq!(get_health_emoji(55), "🟠");
        assert_eq!(get_health_emoji(30), "🔴");
    }
}
