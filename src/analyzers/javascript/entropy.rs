use crate::complexity::entropy_core::{EntropyConfig, EntropyScore, UniversalEntropyCalculator};
use crate::complexity::languages::javascript::JavaScriptEntropyAnalyzer as NewJSAnalyzer;
use tree_sitter::Node;

/// JavaScript/TypeScript entropy analyzer (wrapper for new framework)
pub struct JavaScriptEntropyAnalyzer {
    calculator: UniversalEntropyCalculator,
}

impl JavaScriptEntropyAnalyzer {
    pub fn new() -> Self {
        let config = if crate::config::get_entropy_config().enabled {
            EntropyConfig::default()
        } else {
            EntropyConfig {
                enabled: false,
                ..Default::default()
            }
        };
        
        Self {
            calculator: UniversalEntropyCalculator::new(config),
        }
    }

    /// Calculate entropy for a JavaScript/TypeScript function
    pub fn calculate_entropy(&mut self, node: Node, source: &str) -> EntropyScore {
        // Create language-specific analyzer
        let analyzer = NewJSAnalyzer::new(source);
        
        // Use the universal calculator with the language-specific analyzer
        self.calculator.calculate(&analyzer, &node)
    }
}

impl Default for JavaScriptEntropyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}