use crate::organization::GodObjectAnalysis;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DetailedRecommendation {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub estimated_effort: EffortEstimate,
    pub impact: ImpactAssessment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffortEstimate {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    pub complexity_reduction: i32,
    pub maintainability_improvement: i32,
    pub testability_improvement: i32,
    pub risk_reduction: i32,
}

pub fn generate_god_object_recommendation(
    analysis: &GodObjectAnalysis,
    path: &Path,
) -> DetailedRecommendation {
    let file_name = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let mut action_items = vec![
        "Break into smaller, focused modules:".to_string(),
    ];
    
    for split in &analysis.recommended_splits {
        action_items.push(format!(
            "  - {} ({} methods, ~{} lines)",
            split.suggested_name,
            split.methods_to_move.len(),
            split.estimated_lines
        ));
    }
    
    action_items.push("Apply SOLID principles, especially Single Responsibility".to_string());
    action_items.push("Create interfaces/traits for better abstraction".to_string());
    action_items.push("Add comprehensive tests before refactoring".to_string());

    DetailedRecommendation {
        severity: Severity::Critical,
        
        title: format!(
            "🚨 GOD OBJECT: {} ({} methods, {} fields, {} responsibilities)",
            file_name,
            analysis.method_count,
            analysis.field_count,
            analysis.responsibility_count
        ),
        
        description: format!(
            "This file/class has grown too large and handles too many responsibilities. \
             With {} lines and {} total complexity, it's become difficult to maintain, \
             test, and understand. This is the #1 priority for refactoring.",
            analysis.lines_of_code,
            analysis.complexity_sum
        ),
        
        action_items,
        
        estimated_effort: EffortEstimate::High,
        
        impact: ImpactAssessment {
            complexity_reduction: (analysis.complexity_sum / 2) as i32,
            maintainability_improvement: 80,
            testability_improvement: 70,
            risk_reduction: 90,
        },
    }
}

pub fn format_god_object_display(
    analysis: &GodObjectAnalysis,
    path: &Path,
    score: f64,
    rank: usize,
) -> String {
    let mut output = String::new();
    
    output.push_str(&format!(
        "#{} SCORE: {:.1} [🚨 GOD OBJECT]\n",
        rank, score
    ));
    
    output.push_str(&format!(
        "   └─ {}\n",
        path.display()
    ));
    
    output.push_str("\n   📊 GOD OBJECT METRICS:\n");
    output.push_str(&format!("   ├─ Methods: {} (max: 20)\n", analysis.method_count));
    output.push_str(&format!("   ├─ Fields: {} (max: 15)\n", analysis.field_count));
    output.push_str(&format!("   ├─ Responsibilities: {} (max: 3)\n", analysis.responsibility_count));
    output.push_str(&format!("   ├─ Lines: {} (max: 1,000)\n", analysis.lines_of_code));
    output.push_str(&format!("   └─ Total Complexity: {}\n", analysis.complexity_sum));
    
    if !analysis.recommended_splits.is_empty() {
        output.push_str("\n   🔧 RECOMMENDED REFACTORING:\n");
        output.push_str("   Split into focused modules:\n");
        
        for split in &analysis.recommended_splits {
            output.push_str(&format!(
                "   ├─ {} (~{} lines, {} methods)\n",
                split.suggested_name,
                split.estimated_lines,
                split.methods_to_move.len()
            ));
            output.push_str(&format!(
                "   │  └─ {}\n",
                split.responsibility
            ));
        }
    }
    
    output.push_str(&format!(
        "\n   ⚡ IMPACT: -{} complexity, +80% maintainability, +70% testability\n",
        analysis.complexity_sum / 2
    ));
    
    output
}