/// Format score based on its magnitude
///
/// Scores < 10: Show 2 decimal places (e.g., "9.87")
/// Scores 10-100: Show 1 decimal place (e.g., "45.3")
/// Scores >= 100: Show no decimal places (e.g., "234")
pub fn format_score(score: f64) -> String {
    if score < 10.0 {
        format!("{:.2}", score)
    } else if score < 100.0 {
        format!("{:.1}", score)
    } else {
        format!("{:.0}", score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_score_small() {
        assert_eq!(format_score(3.456), "3.46");
        assert_eq!(format_score(9.999), "10.00");
        assert_eq!(format_score(0.123), "0.12");
    }

    #[test]
    fn test_format_score_medium() {
        assert_eq!(format_score(10.0), "10.0");
        assert_eq!(format_score(45.678), "45.7");
        assert_eq!(format_score(99.99), "100.0");
    }

    #[test]
    fn test_format_score_large() {
        assert_eq!(format_score(100.0), "100");
        assert_eq!(format_score(234.567), "235");
        assert_eq!(format_score(999.999), "1000");
    }
}
