//! Cognitive complexity calculation functions.
//!
//! This module provides functions for calculating cognitive complexity,
//! which measures the mental effort required to understand code.

use syn::Block;

/// Calculate cognitive complexity for a block.
/// This is the main entry point for cognitive complexity calculation.
pub fn calculate_cognitive(block: &Block) -> u32 {
    crate::complexity::calculate_cognitive_for_block(block)
}

/// Calculate the nesting penalty for a given nesting level.
/// Deeper nesting increases cognitive load non-linearly.
///
/// - Level 0: 0 (no penalty)
/// - Level 1: 1
/// - Level 2: 2
/// - Level 3+: 4 or 8 (capped)
pub fn calculate_cognitive_penalty(nesting_level: u32) -> u32 {
    match nesting_level {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        _ => 8, // Cap at 8 for very deep nesting
    }
}

/// Combine multiple cognitive complexity values.
/// Simply sums all complexity values.
pub fn combine_cognitive(complexities: Vec<u32>) -> u32 {
    complexities.into_iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_penalty_levels() {
        assert_eq!(calculate_cognitive_penalty(0), 0);
        assert_eq!(calculate_cognitive_penalty(1), 1);
        assert_eq!(calculate_cognitive_penalty(2), 2);
        assert_eq!(calculate_cognitive_penalty(3), 4);
        assert_eq!(calculate_cognitive_penalty(4), 8);
        assert_eq!(calculate_cognitive_penalty(100), 8);
    }

    #[test]
    fn test_combine_empty() {
        assert_eq!(combine_cognitive(vec![]), 0);
    }

    #[test]
    fn test_combine_values() {
        assert_eq!(combine_cognitive(vec![1, 2, 3]), 6);
    }
}
