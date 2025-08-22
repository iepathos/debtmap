
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_parameter_list_impact_low() {
        // Test low impact for 7 or fewer parameters
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(0),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(5),
            MaintainabilityImpact::Low
        );
        assert_eq!(
            ParameterAnalyzer::classify_parameter_list_impact(7),
            MaintainabilityImpact::Low
        );
    }

    #[test]
    fn test_classify_data_clump_impact_medium() {
        // Test medium impact for more than 5 occurrences
        assert_eq!(
            ParameterAnalyzer::classify_data_clump_impact(6),
            MaintainabilityImpact::Medium
        );
    }
}
