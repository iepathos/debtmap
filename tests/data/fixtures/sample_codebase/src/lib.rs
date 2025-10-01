// Sample codebase for testing debtmap compare functionality

/// A simple function with some complexity
pub fn complex_function(x: i32, y: i32, z: i32) -> i32 {
    if x > 0 {
        if y > 0 {
            if z > 0 {
                return x + y + z;
            } else {
                return x + y - z;
            }
        } else {
            return x - y;
        }
    } else {
        return -x;
    }
}

/// Another function with high cyclomatic complexity
pub fn calculate_score(value: i32) -> String {
    if value > 100 {
        "excellent".to_string()
    } else if value > 80 {
        "good".to_string()
    } else if value > 60 {
        "average".to_string()
    } else if value > 40 {
        "below_average".to_string()
    } else {
        "poor".to_string()
    }
}

/// A well-structured simple function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
