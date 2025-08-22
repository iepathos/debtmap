use debtmap::complexity::{
    if_else_analyzer::IfElseChainAnalyzer,
    message_generator::{format_enhanced_message, generate_enhanced_message},
    recursive_detector::RecursiveMatchDetector,
    threshold_manager::ComplexityThresholds,
};
use debtmap::core::FunctionMetrics;
use std::path::PathBuf;
use syn::parse_quote;

fn main() {
    // Example 1: Function with nested match expressions
    let block: syn::Block = parse_quote! {{
        match state {
            State::Init => {
                match sub_state {
                    SubState::Ready => {
                        match config.mode {
                            Mode::Fast => process_fast(),
                            Mode::Slow => process_slow(),
                            Mode::Auto => auto_select(),
                        }
                    }
                    SubState::Waiting => wait(),
                    SubState::Error => handle_error(),
                }
            }
            State::Running => run(),
            State::Done => cleanup(),
        }
    }};

    // Detect all matches recursively
    let mut detector = RecursiveMatchDetector::new();
    let matches = detector.find_matches_in_block(&block);

    println!("🔍 Recursive Match Detection Demo");
    println!("==================================\n");
    println!("Found {} match expressions:", matches.len());
    for (i, m) in matches.iter().enumerate() {
        println!(
            "  {}. Match with {} arms at depth {}, complexity: {}",
            i + 1,
            m.arms,
            m.context.nesting_depth,
            m.complexity
        );
    }

    // Example 2: Long if-else chain that should be refactored
    let if_else_block: syn::Block = parse_quote! {{
        if file_type == "rust" {
            return ".rs";
        } else if file_type == "python" {
            return ".py";
        } else if file_type == "javascript" {
            return ".js";
        } else if file_type == "typescript" {
            return ".ts";
        } else if file_type == "go" {
            return ".go";
        } else {
            return ".txt";
        }
    }};

    let mut if_analyzer = IfElseChainAnalyzer::new();
    let chains = if_analyzer.analyze_block(&if_else_block);

    println!("\n📊 If-Else Chain Analysis");
    println!("=========================\n");
    for chain in &chains {
        println!("Chain with {} conditions", chain.length);
        println!("  Variable tested: {:?}", chain.variable_tested);
        println!("  Has final else: {}", chain.has_final_else);
        println!(
            "  Suggested refactoring: {}",
            chain.suggest_refactoring().name()
        );
        println!("  {}", chain.suggest_refactoring().description());
    }

    // Example 3: Generate enhanced complexity message
    let mut metrics = FunctionMetrics::new(
        "process_request".to_string(),
        PathBuf::from("src/handler.rs"),
        42,
    );
    metrics.cyclomatic = 15;
    metrics.cognitive = 25;
    metrics.length = 120;

    let thresholds = ComplexityThresholds::default();
    let message = generate_enhanced_message(&metrics, &matches, &chains, &thresholds);

    println!("\n💡 Enhanced Complexity Message");
    println!("==============================");
    println!("{}", format_enhanced_message(&message));

    // Example 4: Demonstrate threshold filtering
    println!("\n⚙️ Complexity Threshold Demo");
    println!("============================\n");

    let mut simple_func =
        FunctionMetrics::new("get_name".to_string(), PathBuf::from("src/utils.rs"), 10);
    simple_func.cyclomatic = 2;
    simple_func.cognitive = 3;
    simple_func.length = 5;

    let mut complex_func = FunctionMetrics::new(
        "process_data".to_string(),
        PathBuf::from("src/processor.rs"),
        100,
    );
    complex_func.cyclomatic = 20;
    complex_func.cognitive = 35;
    complex_func.length = 200;

    println!(
        "Simple function '{}' flagged: {}",
        simple_func.name,
        thresholds.should_flag_function(
            &simple_func,
            debtmap::complexity::threshold_manager::FunctionRole::Utility
        )
    );

    println!(
        "Complex function '{}' flagged: {}",
        complex_func.name,
        thresholds.should_flag_function(
            &complex_func,
            debtmap::complexity::threshold_manager::FunctionRole::CoreLogic
        )
    );

    println!("\nComplexity levels:");
    println!(
        "  {} - {:?}",
        simple_func.name,
        thresholds.get_complexity_level(&simple_func)
    );
    println!(
        "  {} - {:?}",
        complex_func.name,
        thresholds.get_complexity_level(&complex_func)
    );

    // Example 5: Show different threshold presets
    println!("\n📋 Threshold Presets");
    println!("====================\n");

    for preset_name in &["strict", "balanced", "lenient"] {
        if let Some(preset) = ComplexityThresholds::preset(preset_name) {
            println!("{} preset:", preset_name);
            println!(
                "  Min total complexity: {}",
                preset.minimum_total_complexity
            );
            println!("  Min cyclomatic: {}", preset.minimum_cyclomatic_complexity);
            println!("  Min cognitive: {}", preset.minimum_cognitive_complexity);
            println!("  Min match arms: {}", preset.minimum_match_arms);
            println!("  Min if-else chain: {}", preset.minimum_if_else_chain);
            println!();
        }
    }
}
