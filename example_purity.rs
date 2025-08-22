// Example Rust file to demonstrate purity detection

// Pure function - no side effects
fn calculate_tax(amount: f64, rate: f64) -> f64 {
    amount * rate
}

// Pure function - only uses parameters
fn find_max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

// Impure function - performs I/O
fn log_calculation(x: i32, y: i32) -> i32 {
    println!("Calculating {} + {}", x, y);
    x + y
}

// Impure function - modifies external state
fn increment_counter(counter: &mut i32) {
    *counter += 1;
}

// Pure function - complex but no side effects
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// Impure function - file I/O
fn save_to_file(content: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create("output.txt")?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

// Pure function - string manipulation
fn reverse_string(s: &str) -> String {
    s.chars().rev().collect()
}

fn main() {
    println!("Testing purity detection");
}