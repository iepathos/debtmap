// This file demonstrates extreme technical debt that results in scores > 10.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ExtremeLegacyProcessor {
    data: Arc<Mutex<HashMap<String, Vec<ComplexData>>>>,
    cache: Arc<Mutex<HashMap<String, ProcessingResult>>>,
    state: Arc<Mutex<SystemState>>,
}

#[allow(dead_code)]
struct ComplexData {
    id: String,
    values: Vec<f64>,
    metadata: HashMap<String, String>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ProcessingResult {
    score: f64,
    errors: Vec<String>,
    timestamp: u64,
}

struct SystemState {
    running: bool,
    error_count: u32,
    processed: u64,
}

impl ExtremeLegacyProcessor {
    // Extremely complex function with multiple nested conditions
    // This would score very high due to:
    // - High cyclomatic complexity
    // - Deep nesting
    // - Multiple responsibilities
    // - No test coverage
    pub fn process_all_data_with_extreme_complexity(
        &mut self,
    ) -> Result<Vec<ProcessingResult>, String> {
        let mut results = Vec::new();

        if let Ok(data) = self.data.lock() {
            for (key, items) in data.iter() {
                if key.starts_with("process_") {
                    for item in items {
                        if !item.values.is_empty() {
                            let mut score = 0.0;
                            let mut error_count = 0;

                            // Deep nesting level 1
                            for value in &item.values {
                                if *value > 0.0 {
                                    // Deep nesting level 2
                                    if *value < 100.0 {
                                        // Deep nesting level 3
                                        if *value % 2.0 == 0.0 {
                                            // Deep nesting level 4
                                            if item.metadata.contains_key("special") {
                                                // Deep nesting level 5
                                                if let Some(special) = item.metadata.get("special")
                                                {
                                                    // Deep nesting level 6
                                                    if special == "true" {
                                                        score += value * 2.0;
                                                    } else if special == "false" {
                                                        score += value * 0.5;
                                                    } else if special == "maybe" {
                                                        // Deep nesting level 7
                                                        if let Ok(state) = self.state.lock() {
                                                            if state.running {
                                                                if state.error_count < 10 {
                                                                    score += value * 1.5;
                                                                } else if state.error_count < 100 {
                                                                    score += value * 0.75;
                                                                } else {
                                                                    error_count += 1;
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        // More branches
                                                        match special.as_str() {
                                                            "high" => score += value * 3.0,
                                                            "medium" => score += value * 2.0,
                                                            "low" => score += value * 1.0,
                                                            _ => error_count += 1,
                                                        }
                                                    }
                                                }
                                            } else {
                                                // Another condition branch
                                                if item.metadata.contains_key("priority") {
                                                    if let Some(priority) =
                                                        item.metadata.get("priority")
                                                    {
                                                        match priority.parse::<u32>() {
                                                            Ok(p) if p > 5 => score += value * 2.5,
                                                            Ok(p) if p > 3 => score += value * 1.8,
                                                            Ok(_) => score += value,
                                                            Err(_) => error_count += 1,
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // Odd values
                                            if *value > 50.0 {
                                                score += value * 0.3;
                                            } else {
                                                score += value * 0.1;
                                            }
                                        }
                                    } else if *value < 1000.0 {
                                        // Medium range
                                        score += value * 0.01;
                                    } else {
                                        // High range
                                        score += value * 0.001;
                                    }
                                } else if *value < 0.0 {
                                    // Negative values
                                    error_count += 1;
                                }
                            }

                            // Store result
                            if let Ok(mut cache) = self.cache.lock() {
                                let result = ProcessingResult {
                                    score,
                                    errors: vec![format!("Errors: {}", error_count)],
                                    timestamp: 0,
                                };
                                cache.insert(key.clone(), result.clone());
                                results.push(result);
                            }
                        }
                    }
                } else if key.starts_with("special_") {
                    // Another complex branch
                    for item in items {
                        // More processing
                        let mut temp_score = 0.0;
                        for (k, v) in &item.metadata {
                            if k.contains("score") {
                                if let Ok(parsed) = v.parse::<f64>() {
                                    temp_score += parsed;
                                }
                            }
                        }
                        results.push(ProcessingResult {
                            score: temp_score,
                            errors: vec![],
                            timestamp: 0,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    // Another complex function with multiple responsibilities
    pub fn validate_and_transform_data(&mut self, input: Vec<String>) -> Result<(), String> {
        for line in input {
            if line.starts_with("VALIDATE:") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 2 {
                    let command = parts[1];
                    let data = parts[2];

                    match command {
                        "CHECK" => {
                            if data.len() > 10 && data.contains("ERROR") {
                                return Err("Validation error".to_string());
                            }
                        }
                        "TRANSFORM" => {
                            if let Ok(mut state) = self.state.lock() {
                                state.processed += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() {
    println!("Extreme debt example - demonstrating high technical debt scores");
}
