//! Performance benchmarks for AST-based constructor detection (Spec 122)
//!
//! Measures the overhead of AST-based detection vs name-only detection.
//! Target: < 5% overhead for AST analysis.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::analyzers::rust_constructor_detector::{
    analyze_function_body, extract_return_type, ConstructorReturnType,
};
use syn::{parse_quote, ItemFn};

/// Generate Rust code with various constructor patterns
fn generate_constructor_code(num_funcs: usize) -> Vec<ItemFn> {
    let mut functions = Vec::new();

    for i in 0..num_funcs {
        // Standard constructor
        functions.push(parse_quote! {
            pub fn new() -> Self {
                Self { field: 0 }
            }
        });

        // Non-standard constructor (AST should catch)
        functions.push(parse_quote! {
            pub fn create_default() -> Self {
                Self { field: #i }
            }
        });

        // Result-based constructor
        functions.push(parse_quote! {
            pub fn try_new() -> Result<Self, Error> {
                Ok(Self { field: #i })
            }
        });

        // Builder method (should NOT be constructor)
        functions.push(parse_quote! {
            pub fn set_timeout(mut self, timeout: Duration) -> Self {
                self.timeout = timeout;
                self
            }
        });

        // Non-constructor (should NOT match)
        functions.push(parse_quote! {
            pub fn get_value(&self) -> i32 {
                self.field
            }
        });
    }

    functions
}

/// Benchmark AST-based return type extraction
fn bench_extract_return_type(c: &mut Criterion) {
    let functions = generate_constructor_code(20);

    c.bench_function("ast_extract_return_type", |b| {
        b.iter(|| {
            for func in &functions {
                black_box(extract_return_type(func));
            }
        })
    });
}

/// Benchmark full AST body pattern analysis
fn bench_analyze_function_body(c: &mut Criterion) {
    let functions = generate_constructor_code(20);

    c.bench_function("ast_analyze_function_body", |b| {
        b.iter(|| {
            for func in &functions {
                black_box(analyze_function_body(func));
            }
        })
    });
}

/// Benchmark complete AST-based constructor detection
fn bench_full_ast_detection(c: &mut Criterion) {
    let functions = generate_constructor_code(20);

    c.bench_function("ast_full_constructor_detection", |b| {
        b.iter(|| {
            for func in &functions {
                let return_type = extract_return_type(func);
                let is_self_return = matches!(
                    return_type,
                    Some(
                        ConstructorReturnType::OwnedSelf
                            | ConstructorReturnType::ResultSelf
                            | ConstructorReturnType::OptionSelf
                    )
                );

                if is_self_return {
                    let pattern = analyze_function_body(func);
                    black_box(pattern.is_constructor_like());
                }
            }
        })
    });
}

/// Benchmark name-only detection (baseline)
fn bench_name_only_detection(c: &mut Criterion) {
    let functions = generate_constructor_code(20);

    c.bench_function("name_only_constructor_detection", |b| {
        b.iter(|| {
            for func in &functions {
                let name = func.sig.ident.to_string();
                let is_constructor = name == "new"
                    || name.starts_with("new_")
                    || name.starts_with("try_new")
                    || name.starts_with("from_");
                black_box(is_constructor);
            }
        })
    });
}

/// Benchmark overhead: AST vs name-only
fn bench_overhead_comparison(c: &mut Criterion) {
    let functions = generate_constructor_code(50);

    c.bench_function("comparison_name_only", |b| {
        b.iter(|| {
            let mut count = 0;
            for func in &functions {
                let name = func.sig.ident.to_string();
                if name == "new" || name.starts_with("new_") {
                    count += 1;
                }
            }
            black_box(count)
        })
    });

    c.bench_function("comparison_ast_based", |b| {
        b.iter(|| {
            let mut count = 0;
            for func in &functions {
                if let Some(return_type) = extract_return_type(func) {
                    if matches!(
                        return_type,
                        ConstructorReturnType::OwnedSelf
                            | ConstructorReturnType::ResultSelf
                            | ConstructorReturnType::OptionSelf
                    ) {
                        let pattern = analyze_function_body(func);
                        if pattern.is_constructor_like() {
                            count += 1;
                        }
                    }
                }
            }
            black_box(count)
        })
    });
}

/// Benchmark scalability with different codebase sizes
fn bench_scalability(c: &mut Criterion) {
    let sizes = vec![("small", 10), ("medium", 50), ("large", 200)];

    for (name, num_funcs) in sizes {
        let functions = generate_constructor_code(num_funcs);

        c.bench_function(&format!("ast_detection_{name}"), |b| {
            b.iter(|| {
                for func in &functions {
                    if let Some(return_type) = extract_return_type(func) {
                        if matches!(
                            return_type,
                            ConstructorReturnType::OwnedSelf
                                | ConstructorReturnType::ResultSelf
                                | ConstructorReturnType::OptionSelf
                        ) {
                            let pattern = analyze_function_body(func);
                            black_box(pattern.is_constructor_like());
                        }
                    }
                }
            })
        });

        c.bench_function(&format!("name_only_{name}"), |b| {
            b.iter(|| {
                for func in &functions {
                    let name = func.sig.ident.to_string();
                    black_box(
                        name == "new"
                            || name.starts_with("new_")
                            || name.starts_with("try_new"),
                    );
                }
            })
        });
    }
}

/// Benchmark realistic mixed codebase
fn bench_realistic_mixed_code(c: &mut Criterion) {
    let functions: Vec<ItemFn> = vec![
        // Constructors (should match)
        parse_quote! { pub fn new() -> Self { Self { timeout: 30 } } },
        parse_quote! { pub fn try_new(val: i32) -> Result<Self, Error> { Ok(Self { val }) } },
        parse_quote! { pub fn create_default() -> Self { Self::new() } },
        // Non-constructors (should NOT match)
        parse_quote! { pub fn get_value(&self) -> i32 { self.val } },
        parse_quote! { pub fn process(&mut self) { self.val += 1; } },
        parse_quote! {
            pub fn complex_logic(x: i32) -> i32 {
                if x > 0 {
                    for i in 0..x {
                        println!("{}", i);
                    }
                }
                x * 2
            }
        },
        // Builder methods (should NOT match as constructors)
        parse_quote! {
            pub fn set_timeout(mut self, timeout: Duration) -> Self {
                self.timeout = timeout;
                self
            }
        },
        parse_quote! {
            pub fn with_config(mut self, config: Config) -> Self {
                self.config = config;
                self
            }
        },
    ];

    c.bench_function("realistic_mixed_ast", |b| {
        b.iter(|| {
            for func in &functions {
                if let Some(return_type) = extract_return_type(func) {
                    if matches!(
                        return_type,
                        ConstructorReturnType::OwnedSelf
                            | ConstructorReturnType::ResultSelf
                            | ConstructorReturnType::OptionSelf
                    ) {
                        let pattern = analyze_function_body(func);
                        black_box(pattern.is_constructor_like());
                    }
                }
            }
        })
    });

    c.bench_function("realistic_mixed_name_only", |b| {
        b.iter(|| {
            for func in &functions {
                let name = func.sig.ident.to_string();
                black_box(
                    name == "new"
                        || name.starts_with("new_")
                        || name.starts_with("try_new")
                        || name.starts_with("from_"),
                );
            }
        })
    });
}

criterion_group!(
    constructor_benches,
    bench_extract_return_type,
    bench_analyze_function_body,
    bench_full_ast_detection,
    bench_name_only_detection,
    bench_overhead_comparison,
    bench_scalability,
    bench_realistic_mixed_code
);

criterion_main!(constructor_benches);
