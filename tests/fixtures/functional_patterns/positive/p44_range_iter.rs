fn range_sum(start: i32, end: i32) -> i32 {
    (start..end)
        .filter(|&x| x % 2 == 0)
        .sum()
}
