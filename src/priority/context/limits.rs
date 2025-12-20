//! Line limit and prioritization for context suggestions (Spec 263).

use super::types::{ContextSuggestion, RelatedContext};

/// Apply line limits to context suggestions.
pub fn apply_limits(mut suggestion: ContextSuggestion, max_total_lines: u32) -> ContextSuggestion {
    let primary_lines = suggestion.primary.line_count();

    if primary_lines >= max_total_lines {
        suggestion.related.clear();
        suggestion.total_lines = primary_lines;
        suggestion.completeness_confidence *= 0.5;
        return suggestion;
    }

    let remaining_budget = max_total_lines - primary_lines;

    suggestion.related.sort_by_key(priority_score);
    suggestion.related.reverse();

    let mut used_lines = 0u32;
    let mut kept_contexts = Vec::new();

    for ctx in suggestion.related {
        let ctx_lines = ctx.range.line_count();
        if used_lines + ctx_lines <= remaining_budget {
            used_lines += ctx_lines;
            kept_contexts.push(ctx);
        } else {
            suggestion.completeness_confidence *= 0.8;
            break;
        }
    }

    suggestion.related = kept_contexts;
    suggestion.total_lines = primary_lines + used_lines;
    suggestion
}

fn priority_score(ctx: &RelatedContext) -> u32 {
    use super::types::ContextRelationship;

    let base = match ctx.relationship {
        ContextRelationship::ModuleHeader => 100,
        ContextRelationship::Caller => 80,
        ContextRelationship::Callee => 70,
        ContextRelationship::TestCode => 60,
        ContextRelationship::TypeDefinition => 50,
        ContextRelationship::TraitDefinition => 40,
        ContextRelationship::SiblingMethod => 30,
    };

    let lines = ctx.range.line_count();
    if lines > 100 {
        base / 2
    } else if lines > 50 {
        base * 3 / 4
    } else {
        base
    }
}
