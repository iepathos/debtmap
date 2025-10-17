//! Performance benchmarks for Python import resolution
//!
//! Measures import resolution performance and cache effectiveness to verify:
//! - Cache hit rate > 90%
//! - Performance improvement > 20% with caching

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::analysis::python_imports::{EnhancedImportResolver, ImportType};
use rustpython_parser as rp;
use std::hint::black_box;
use std::path::{Path, PathBuf};

/// Generate a Python module with various import patterns
fn generate_module_with_imports(
    num_direct: usize,
    num_from: usize,
    num_star: usize,
    num_relative: usize,
) -> String {
    let mut code = String::new();

    // Direct imports
    for i in 0..num_direct {
        code.push_str(&format!("import module_{}\n", i));
    }

    // From imports
    for i in 0..num_from {
        code.push_str(&format!("from module_{} import func_{}\n", i, i));
    }

    // Star imports
    for i in 0..num_star {
        code.push_str(&format!("from module_{} import *\n", i + 100));
    }

    // Relative imports
    for i in 0..num_relative {
        let level = (i % 3) + 1;
        code.push_str(&format!(
            "from {}.module_{} import func_{}\n",
            ".".repeat(level),
            i,
            i
        ));
    }

    // Add some function definitions
    code.push_str("\ndef my_function():\n    pass\n\n");
    code.push_str("class MyClass:\n    def method(self):\n        pass\n");

    code
}

/// Generate a module with dynamic imports
fn generate_module_with_dynamic_imports(num_dynamic: usize) -> String {
    let mut code = String::new();

    code.push_str("import importlib\n\n");

    // __import__() calls
    for i in 0..num_dynamic / 2 {
        code.push_str(&format!(
            "module_{} = __import__('dynamic_module_{}')\n",
            i, i
        ));
    }

    // importlib.import_module() calls
    for i in num_dynamic / 2..num_dynamic {
        code.push_str(&format!(
            "module_{} = importlib.import_module('dynamic_module_{}')\n",
            i, i
        ));
    }

    code.push_str("\ndef load_plugin(name):\n");
    code.push_str("    return __import__(name)\n\n");

    code.push_str("def load_module_dynamically(name):\n");
    code.push_str("    return importlib.import_module(name)\n");

    code
}

/// Generate interconnected modules for graph building
fn generate_module_graph(num_modules: usize) -> Vec<(String, PathBuf)> {
    let mut modules = Vec::new();

    for i in 0..num_modules {
        let mut code = String::new();

        // Import from 2-3 other modules
        let imports_per_module = 3;
        for j in 0..imports_per_module {
            let target = (i + j + 1) % num_modules;
            code.push_str(&format!("from module_{} import func_{}\n", target, target));
        }

        // Define some symbols
        code.push_str(&format!("\ndef func_{}():\n    pass\n\n", i));
        code.push_str(&format!("class Class_{}:\n    pass\n\n", i));

        let path = PathBuf::from(format!("test_project/module_{}.py", i));
        modules.push((code, path));
    }

    modules
}

/// Benchmark basic import analysis
fn bench_import_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("import_analysis");

    for (size_name, num_imports) in &[("small", 10), ("medium", 50), ("large", 200)] {
        let code = generate_module_with_imports(*num_imports / 2, *num_imports / 4, 5, 10);

        group.bench_with_input(BenchmarkId::from_parameter(size_name), size_name, |b, _| {
            b.iter(|| {
                let module =
                    rp::parse(&code, rp::Mode::Module, "test.py").expect("Failed to parse");
                let mut resolver = EnhancedImportResolver::new();
                resolver.analyze_imports(&module, Path::new("test.py"));
                black_box(())
            })
        });
    }

    group.finish();
}

/// Benchmark dynamic import detection
fn bench_dynamic_import_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_import_detection");

    for (size_name, num_dynamic) in &[("few", 5), ("many", 20), ("extensive", 50)] {
        let code = generate_module_with_dynamic_imports(*num_dynamic);

        group.bench_with_input(BenchmarkId::from_parameter(size_name), size_name, |b, _| {
            b.iter(|| {
                let module =
                    rp::parse(&code, rp::Mode::Module, "test.py").expect("Failed to parse");
                let mut resolver = EnhancedImportResolver::new();
                resolver.analyze_imports(&module, Path::new("test.py"));

                // Count dynamic imports
                let edges = resolver.import_graph().edges.get(Path::new("test.py"));
                let dynamic_count = edges
                    .map(|e| {
                        e.iter()
                            .filter(|edge| edge.import_type == ImportType::Dynamic)
                            .count()
                    })
                    .unwrap_or(0);

                black_box(dynamic_count)
            })
        });
    }

    group.finish();
}

/// Benchmark import graph building with multiple modules
fn bench_graph_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_building");

    for (size_name, num_modules) in &[("small", 10), ("medium", 50), ("large", 100)] {
        let modules_data = generate_module_graph(*num_modules);

        group.bench_with_input(BenchmarkId::from_parameter(size_name), size_name, |b, _| {
            b.iter(|| {
                let mut resolver = EnhancedImportResolver::new();
                let parsed_modules: Vec<_> = modules_data
                    .iter()
                    .map(|(code, path)| {
                        let module =
                            rp::parse(code, rp::Mode::Module, "test.py").expect("Failed to parse");
                        (module, path.clone())
                    })
                    .collect();

                resolver.build_import_graph(&parsed_modules);
                black_box(())
            })
        });
    }

    group.finish();
}

/// Benchmark symbol resolution with caching
fn bench_symbol_resolution_with_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_resolution_cache");

    // Create a module with many symbols
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("def func_{}():\n    pass\n\n", i));
    }
    for i in 0..50 {
        code.push_str(&format!("class Class_{}:\n    pass\n\n", i));
    }

    let module = rp::parse(&code, rp::Mode::Module, "test.py").expect("Failed to parse");
    let mut resolver = EnhancedImportResolver::new();
    resolver.analyze_imports(&module, Path::new("test.py"));

    // Benchmark repeated symbol resolution (tests cache effectiveness)
    group.bench_function("repeated_lookups", |b| {
        b.iter(|| {
            // Look up each symbol twice to test caching
            for i in 0..50 {
                let symbol_name = format!("func_{}", i);
                black_box(resolver.resolve_symbol(Path::new("test.py"), &symbol_name));
                black_box(resolver.resolve_symbol(Path::new("test.py"), &symbol_name));
            }
        })
    });

    group.finish();
}

/// Benchmark symbol resolution without caching (for comparison)
fn bench_symbol_resolution_no_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_resolution_no_cache");

    // Create a module with many symbols
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("def func_{}():\n    pass\n\n", i));
    }

    let module = rp::parse(&code, rp::Mode::Module, "test.py").expect("Failed to parse");

    // Benchmark symbol resolution with fresh resolver each time (no caching benefit)
    group.bench_function("fresh_resolver", |b| {
        b.iter(|| {
            let mut resolver = EnhancedImportResolver::new();
            resolver.analyze_imports(&module, Path::new("test.py"));

            for i in 0..50 {
                let symbol_name = format!("func_{}", i);
                black_box(resolver.resolve_symbol(Path::new("test.py"), &symbol_name));
            }
        })
    });

    group.finish();
}

/// Benchmark circular import detection
fn bench_circular_import_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("circular_import_detection");

    // Create modules with circular dependencies
    for (size_name, chain_length) in &[("short", 3), ("medium", 10), ("long", 20)] {
        let mut modules = Vec::new();

        for i in 0..*chain_length {
            let next = (i + 1) % chain_length;
            let code = format!(
                "from module_{} import func_{}\n\ndef func_{}():\n    pass\n",
                next, next, i
            );
            let path = PathBuf::from(format!("test_project/module_{}.py", i));
            modules.push((code, path));
        }

        group.bench_with_input(BenchmarkId::from_parameter(size_name), size_name, |b, _| {
            b.iter(|| {
                let mut resolver = EnhancedImportResolver::new();
                let parsed_modules: Vec<_> = modules
                    .iter()
                    .map(|(code, path)| {
                        let module =
                            rp::parse(code, rp::Mode::Module, "test.py").expect("Failed to parse");
                        (module, path.clone())
                    })
                    .collect();

                resolver.build_import_graph(&parsed_modules);
                let cycles = resolver.circular_imports().to_vec();
                black_box(cycles)
            })
        });
    }

    group.finish();
}

/// Benchmark realistic mixed import patterns
fn bench_realistic_patterns(c: &mut Criterion) {
    let code = r#"
import os
import sys as system
from typing import List, Dict, Optional
from collections import *
from . import helper
from .. import utils
import importlib

# Dynamic imports
json_module = __import__('json')
plugin = importlib.import_module('plugins.core')

def load_config(path: str) -> Dict:
    with open(path) as f:
        return json_module.load(f)

def dynamic_load(module_name: str):
    return importlib.import_module(module_name)

class ConfigManager:
    def __init__(self):
        self.settings = {}

    def load_plugin(self, name: str):
        plugin = __import__(name)
        return plugin.initialize(self.settings)

    def get_setting(self, key: str) -> Optional[str]:
        return self.settings.get(key)

def main():
    manager = ConfigManager()
    config = load_config('config.json')
    manager.settings = config

    for plugin_name in config.get('plugins', []):
        manager.load_plugin(plugin_name)
"#;

    c.bench_function("realistic_import_patterns", |b| {
        b.iter(|| {
            let module = rp::parse(code, rp::Mode::Module, "test.py").expect("Failed to parse");
            let mut resolver = EnhancedImportResolver::new();
            resolver.analyze_imports(&module, Path::new("test.py"));

            // Resolve some symbols
            black_box(resolver.resolve_symbol(Path::new("test.py"), "ConfigManager"));
            black_box(resolver.resolve_symbol(Path::new("test.py"), "load_config"));
            black_box(resolver.resolve_symbol(Path::new("test.py"), "main"));
        })
    });
}

/// Benchmark large-scale analysis (spec requirement: <2s for 1000 files)
fn bench_large_scale_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scale_performance");
    group.sample_size(10); // Reduce sample size for large benchmarks
    group.measurement_time(std::time::Duration::from_secs(30));

    // Test with 1000 files as per spec requirement
    let num_files = 1000;
    let modules_data = generate_module_graph(num_files);

    group.bench_function("1000_files", |b| {
        b.iter(|| {
            let mut resolver = EnhancedImportResolver::new();
            let parsed_modules: Vec<_> = modules_data
                .iter()
                .map(|(code, path)| {
                    let module = rp::parse(code, rp::Mode::Module, path.to_str().unwrap())
                        .expect("Failed to parse");
                    (module, path.clone())
                })
                .collect();

            resolver.build_import_graph(&parsed_modules);

            // Perform some symbol resolution to test complete workflow
            for i in 0..10 {
                let symbol_name = format!("func_{}", i);
                let module_path = format!("test_project/module_{}.py", i);
                black_box(resolver.resolve_symbol(Path::new(&module_path), &symbol_name));
            }

            black_box(())
        })
    });

    group.finish();
}

criterion_group!(
    import_resolution_benches,
    bench_import_analysis,
    bench_dynamic_import_detection,
    bench_graph_building,
    bench_symbol_resolution_with_cache,
    bench_symbol_resolution_no_cache,
    bench_circular_import_detection,
    bench_realistic_patterns,
    bench_large_scale_performance
);

criterion_main!(import_resolution_benches);
