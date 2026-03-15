use debtmap::extraction::UnifiedFileExtractor;
use std::path::Path;

#[test]
fn test_python_entropy_repetition_dampening() {
    let source = r#"
def validate_user(user):
    if not user.get('id'): raise ValueError('id missing')
    if not user.get('username'): raise ValueError('username missing')
    if not user.get('email'): raise ValueError('email missing')
    if not user.get('password'): raise ValueError('password missing')
    if not user.get('created_at'): raise ValueError('created_at missing')
"#;
    let path = Path::new("test.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let func = &data.functions[0];
    let entropy = func
        .entropy_score
        .as_ref()
        .expect("Should have entropy score");

    // Cyclomatic complexity is high (6), but repetition should be high too
    assert_eq!(func.cyclomatic, 6);
    assert!(
        entropy.pattern_repetition > 0.6,
        "Expected high repetition, got {}",
        entropy.pattern_repetition
    );
}

#[test]
fn test_python_entropy_complex_algorithm() {
    let source = r#"
def process_data(data):
    results = []
    for item in data:
        if item.get('type') == 'A':
            val = item.get('value', 0) * 2
            results.append(val)
        elif item.get('type') == 'B':
            try:
                res = external_call(item.get('id'))
                results.append(res)
            except Exception:
                results.append(None)
    return results
"#;
    let path = Path::new("test.py");
    let data = UnifiedFileExtractor::extract(path, source).expect("Failed to extract");

    let func = &data.functions[0];
    let entropy = func
        .entropy_score
        .as_ref()
        .expect("Should have entropy score");

    // We want validation to have HIGHER repetition than complex logic
    // Let's check relative difference
    let validation_source = r#"
def validate_user(user):
    if not user.get('id'): raise ValueError('id missing')
    if not user.get('username'): raise ValueError('username missing')
    if not user.get('email'): raise ValueError('email missing')
    if not user.get('password'): raise ValueError('password missing')
    if not user.get('created_at'): raise ValueError('created_at missing')
"#;
    let validation_data =
        UnifiedFileExtractor::extract(path, validation_source).expect("Failed to extract");
    let validation_entropy = validation_data.functions[0].entropy_score.as_ref().unwrap();

    assert!(
        validation_entropy.pattern_repetition > entropy.pattern_repetition,
        "Validation ({}) should be more repetitive than complex logic ({})",
        validation_entropy.pattern_repetition,
        entropy.pattern_repetition
    );
}
