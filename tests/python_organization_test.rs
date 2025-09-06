#[cfg(test)]
mod tests {
    use debtmap::organization::python::SimplifiedPythonOrganizationDetector;
    use debtmap::organization::{
        OrganizationAntiPattern, ParameterRefactoring, PrimitiveUsageContext,
    };
    use rustpython_parser;
    use rustpython_parser::ast;
    use std::path::Path;

    fn parse_python_code(source: &str) -> ast::Mod {
        rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse Python code")
    }

    #[test]
    fn test_god_object_detection() {
        let source = r#"
class BigClass:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
        self.field3 = 3
        self.field4 = 4
        self.field5 = 5
        self.field6 = 6
        self.field7 = 7
        self.field8 = 8
        self.field9 = 9
        self.field10 = 10
        self.field11 = 11
    
    def method1(self): pass
    def method2(self): pass
    def method3(self): pass
    def method4(self): pass
    def method5(self): pass
    def method6(self): pass
    def method7(self): pass
    def method8(self): pass
    def method9(self): pass
    def method10(self): pass
    def method11(self): pass
    def method12(self): pass
    def method13(self): pass
    def method14(self): pass
    def method15(self): pass
    def method16(self): pass
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        assert!(!patterns.is_empty());
        assert!(patterns
            .iter()
            .any(|p| matches!(p, OrganizationAntiPattern::GodObject { .. })));

        if let Some(OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            ..
        }) = patterns
            .iter()
            .find(|p| matches!(p, OrganizationAntiPattern::GodObject { .. }))
        {
            assert_eq!(type_name, "BigClass");
            assert_eq!(*method_count, 16);
            assert_eq!(*field_count, 11);
        }
    }

    #[test]
    fn test_magic_value_detection() {
        let source = r#"
def calculate_price(quantity):
    if quantity > 100:
        return quantity * 0.9
    elif quantity > 50:
        return quantity * 0.95
    else:
        if quantity < 10:
            return quantity * 1.1
        return quantity * 1.0
    
    if quantity == 100:
        return quantity * 0.9
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        let magic_values: Vec<_> = patterns
            .iter()
            .filter_map(|p| match p {
                OrganizationAntiPattern::MagicValue { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect();

        assert!(magic_values.contains(&"100"));
    }

    #[test]
    fn test_long_parameter_list_detection() {
        let source = r#"
def complex_function(param1, param2, param3, param4, param5, param6, param7):
    return param1 + param2
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        assert!(patterns
            .iter()
            .any(|p| matches!(p, OrganizationAntiPattern::LongParameterList { .. })));

        if let Some(OrganizationAntiPattern::LongParameterList {
            function_name,
            parameter_count,
            suggested_refactoring,
            ..
        }) = patterns
            .iter()
            .find(|p| matches!(p, OrganizationAntiPattern::LongParameterList { .. }))
        {
            assert_eq!(function_name, "complex_function");
            assert_eq!(*parameter_count, 7);
            assert_eq!(*suggested_refactoring, ParameterRefactoring::ExtractStruct);
        }
    }

    #[test]
    fn test_feature_envy_detection() {
        let source = r#"
class DataProcessor:
    def process(self, data):
        result = data.clean()
        result = data.normalize()
        result = data.validate()
        result = data.transform()
        result = data.format()
        self.log("Processing complete")
        return result
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        let feature_envy = patterns
            .iter()
            .find(|p| matches!(p, OrganizationAntiPattern::FeatureEnvy { .. }));

        assert!(feature_envy.is_some());
        if let Some(OrganizationAntiPattern::FeatureEnvy {
            method_name,
            envied_type,
            external_calls,
            internal_calls,
            suggested_move,
            ..
        }) = feature_envy
        {
            assert_eq!(method_name, "process");
            assert_eq!(envied_type, "data");
            assert_eq!(*external_calls, 5);
            assert_eq!(*internal_calls, 1);
            assert!(suggested_move);
        }
    }

    #[test]
    fn test_primitive_obsession_detection() {
        let source = r#"
def process_user(user_id, user_status, user_type, order_id, order_status):
    pass

def update_user(user_id, user_status, user_category):
    pass

def validate_order(order_id, order_status, order_type):
    pass
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        let primitive_obsession_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p, OrganizationAntiPattern::PrimitiveObsession { .. }))
            .collect();

        assert!(!primitive_obsession_patterns.is_empty());

        // Should detect identifier and status primitive obsessions
        assert!(primitive_obsession_patterns.iter().any(|p| {
            if let OrganizationAntiPattern::PrimitiveObsession { usage_context, .. } = p {
                *usage_context == PrimitiveUsageContext::Identifier
            } else {
                false
            }
        }));

        assert!(primitive_obsession_patterns.iter().any(|p| {
            if let OrganizationAntiPattern::PrimitiveObsession { usage_context, .. } = p {
                *usage_context == PrimitiveUsageContext::Status
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_data_clump_detection() {
        let source = r#"
def create_order(customer_name, customer_email, customer_phone):
    pass

def update_customer(customer_name, customer_email, customer_phone):
    pass

def validate_customer(customer_name, customer_email, customer_phone):
    pass
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        let data_clumps: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p, OrganizationAntiPattern::DataClump { .. }))
            .collect();

        assert!(!data_clumps.is_empty());

        if let Some(OrganizationAntiPattern::DataClump {
            parameter_group,
            occurrence_count,
            ..
        }) = data_clumps.first()
        {
            assert_eq!(parameter_group.parameters.len(), 3);
            assert_eq!(*occurrence_count, 3);
        }
    }

    #[test]
    fn test_configurable_thresholds() {
        let source = r#"
class SmallClass:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
        self.field3 = 3
    
    def method1(self): pass
    def method2(self): pass
    def method3(self): pass
    def method4(self): pass
    def method5(self): pass
"#;

        let module = parse_python_code(source);

        // With default thresholds - should not detect as God Object
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);
        assert!(!patterns
            .iter()
            .any(|p| matches!(p, OrganizationAntiPattern::GodObject { .. })));

        // With lower thresholds - should detect as God Object
        let detector = SimplifiedPythonOrganizationDetector::with_thresholds(
            4,    // god_object_method_threshold
            2,    // god_object_field_threshold
            5,    // long_parameter_threshold
            2,    // magic_value_min_occurrences
            0.33, // feature_envy_threshold
            3,    // primitive_obsession_min_occurrences
        );
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);
        assert!(patterns
            .iter()
            .any(|p| matches!(p, OrganizationAntiPattern::GodObject { .. })));
    }

    #[test]
    fn test_improved_magic_value_traversal() {
        let source = r#"
def complex_calculations():
    # Magic values in various expression contexts
    result = 42 * 3.14
    if result > 100:
        result = result / 2.0
    
    data = [1, 2, 3, 42, 5]
    mapping = {"key": 42, "value": 100}
    
    # Magic value in ternary expression
    x = 42 if result > 100 else 24
    
    # Magic value in function call
    process(42, threshold=100)
    
    # Magic value in comparison
    while result < 42:
        result += 10
    
    return result + 42
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        let magic_values: Vec<_> = patterns
            .iter()
            .filter_map(|p| match p {
                OrganizationAntiPattern::MagicValue {
                    value,
                    occurrence_count,
                    ..
                } => Some((value.as_str(), *occurrence_count)),
                _ => None,
            })
            .collect();

        // Should detect multiple occurrences of 42 and 100
        assert!(magic_values
            .iter()
            .any(|(val, count)| *val == "42" && *count >= 4));
        assert!(magic_values
            .iter()
            .any(|(val, count)| *val == "100" && *count >= 2));
    }

    #[test]
    fn test_source_location_tracking() {
        let source = r#"
class TestClass:
    pass

def test_function():
    pass
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::with_thresholds(
            0, // Force detection
            0, 0, 1, 0.0, 1,
        );
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        for pattern in patterns {
            let location = pattern.primary_location();
            // Should have found actual line numbers, not just default to line 1
            if source.contains("TestClass")
                && matches!(pattern, OrganizationAntiPattern::GodObject { .. })
            {
                assert_eq!(location.line, 2);
            }
        }
    }

    #[test]
    fn test_empty_module() {
        let source = "";
        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        assert!(patterns.is_empty());
    }

    #[test]
    fn test_no_antipatterns() {
        let source = r#"
class WellDesignedClass:
    def __init__(self):
        self.data = []
    
    def add(self, item):
        self.data.append(item)
    
    def remove(self, item):
        self.data.remove(item)

def simple_function(x, y):
    return x + y
"#;

        let module = parse_python_code(source);
        let detector = SimplifiedPythonOrganizationDetector::new();
        let patterns = detector.detect_patterns(&module, Path::new("test.py"), source);

        assert!(patterns.is_empty());
    }
}
