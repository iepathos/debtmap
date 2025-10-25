fn check_conditions(items: Vec<i32>) -> (bool, bool) {
    let has_positive = items.iter().any(|&x| x > 0);
    let all_positive = items.iter().all(|&x| x > 0);
    (has_positive, all_positive)
}
