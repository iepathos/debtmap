fn describe(x: i32) -> String {
    match x {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        _ => "other".to_string(),
    }
}
