use debtmap::complexity::cognitive::calculate_cognitive;
use debtmap::complexity::cyclomatic::calculate_cyclomatic;

#[test]
fn test_cognitive_vs_cyclomatic_simple_if() {
    let code = r#"
        {
            if x > 0 {
                println!("positive");
            }
        }
    "#;

    let block: syn::Block = syn::parse_str(code).unwrap();
    let cyclo = calculate_cyclomatic(&block);
    let cognitive = calculate_cognitive(&block);

    println!("Simple if - Cyclomatic: {cyclo}, Cognitive: {cognitive}");

    // Cyclomatic should be 2 (base 1 + 1 for if)
    assert_eq!(cyclo, 2);
    // Cognitive should be 1 (1 for if at nesting 0)
    assert_eq!(cognitive, 1);
    assert_ne!(cyclo, cognitive);
}

#[test]
fn test_cognitive_vs_cyclomatic_nested_if() {
    let code = r#"
        {
            if x > 0 {
                if y > 0 {
                    println!("both positive");
                }
            }
        }
    "#;

    let block: syn::Block = syn::parse_str(code).unwrap();
    let cyclo = calculate_cyclomatic(&block);
    let cognitive = calculate_cognitive(&block);

    println!("Nested if - Cyclomatic: {cyclo}, Cognitive: {cognitive}");

    // Cyclomatic should be 3 (base 1 + 1 for each if)
    assert_eq!(cyclo, 3);
    // Cognitive should be 3 (1 for first if + 2 for nested if at level 1)
    assert_eq!(cognitive, 3);
}

#[test]
fn test_cognitive_vs_cyclomatic_match() {
    let code = r#"
        {
            match x {
                1 => println!("one"),
                2 => println!("two"),
                3 => println!("three"),
                _ => println!("other"),
            }
        }
    "#;

    let block: syn::Block = syn::parse_str(code).unwrap();
    let cyclo = calculate_cyclomatic(&block);
    let cognitive = calculate_cognitive(&block);

    println!("Match - Cyclomatic: {cyclo}, Cognitive: {cognitive}");

    // Cyclomatic: base 1 + 3 (for 4 arms - 1)
    assert_eq!(cyclo, 4);
    // Cognitive: Simple pattern-matching match expressions get logarithmic scaling
    // log2(4) = 2 for 4 simple arms (macros like println!)
    assert_eq!(cognitive, 2);
    assert_ne!(cyclo, cognitive);
}
