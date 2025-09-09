use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectAnalysis {
    pub is_god_object: bool,
    pub method_count: usize,
    pub field_count: usize,
    pub responsibility_count: usize,
    pub lines_of_code: usize,
    pub complexity_sum: u32,
    pub god_object_score: f64,
    pub recommended_splits: Vec<ModuleSplit>,
    pub confidence: GodObjectConfidence,
    pub responsibilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GodObjectConfidence {
    Definite,     // Exceeds all thresholds
    Probable,     // Exceeds most thresholds
    Possible,     // Exceeds some thresholds
    NotGodObject, // Within acceptable limits
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    pub responsibility: String,
    pub estimated_lines: usize,
}

#[derive(Debug, Clone)]
pub struct GodObjectThresholds {
    pub max_methods: usize,
    pub max_fields: usize,
    pub max_traits: usize,
    pub max_lines: usize,
    pub max_complexity: u32,
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }
}

impl GodObjectThresholds {
    pub fn for_rust() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_traits: 5,
            max_lines: 1000,
            max_complexity: 200,
        }
    }

    pub fn for_python() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }

    pub fn for_javascript() -> Self {
        Self {
            max_methods: 15,
            max_fields: 20,
            max_traits: 3,
            max_lines: 500,
            max_complexity: 150,
        }
    }
}

pub fn calculate_god_object_score(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if method_count > thresholds.max_methods {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Ensure minimum score of 100 for any god object
    if violation_count > 0 {
        // For any god object (at least 1 violation), ensure minimum 100 points
        // Scale up based on severity of violations
        let min_score = 100.0;
        let severity_multiplier = violation_count as f64;
        (base_score * 50.0 * severity_multiplier).max(min_score)
    } else {
        base_score * 10.0
    }
}

pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if method_count > thresholds.max_methods {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }
    if complexity_sum > thresholds.max_complexity {
        violations += 1;
    }

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}

pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let responsibility = infer_responsibility_from_method(method);
        groups
            .entry(responsibility)
            .or_default()
            .push(method.clone());
    }

    groups
}

fn infer_responsibility_from_method(method_name: &str) -> String {
    let lower = method_name.to_lowercase();

    if lower.starts_with("get") || lower.starts_with("set") {
        "Data Access".to_string()
    } else if lower.starts_with("calculate") || lower.starts_with("compute") {
        "Computation".to_string()
    } else if lower.starts_with("validate")
        || lower.starts_with("check")
        || lower.starts_with("verify")
    {
        "Validation".to_string()
    } else if lower.starts_with("save") || lower.starts_with("load") || lower.starts_with("store") {
        "Persistence".to_string()
    } else if lower.starts_with("create") || lower.starts_with("build") || lower.starts_with("new")
    {
        "Construction".to_string()
    } else if lower.starts_with("send")
        || lower.starts_with("receive")
        || lower.starts_with("handle")
    {
        "Communication".to_string()
    } else if lower.starts_with("process") || lower.starts_with("transform") {
        "Processing".to_string()
    } else {
        "Core Operations".to_string()
    }
}

pub fn recommend_module_splits(
    type_name: &str,
    _methods: &[String],
    responsibility_groups: &HashMap<String, Vec<String>>,
) -> Vec<ModuleSplit> {
    let mut recommendations = Vec::new();

    for (responsibility, methods) in responsibility_groups {
        if methods.len() > 5 {
            recommendations.push(ModuleSplit {
                suggested_name: format!(
                    "{}_{}",
                    type_name.to_lowercase(),
                    responsibility.to_lowercase().replace(' ', "_")
                ),
                methods_to_move: methods.clone(),
                responsibility: responsibility.clone(),
                estimated_lines: methods.len() * 20, // Rough estimate
            });
        }
    }

    recommendations
}
