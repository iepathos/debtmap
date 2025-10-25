fn sum_values(items: Vec<i32>) -> i32 {
    items.iter().fold(0, |acc, x| acc + x)
}
