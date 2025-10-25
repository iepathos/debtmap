fn powers_of_two(n: usize) -> Vec<i32> {
    std::iter::successors(Some(1), |&x| Some(x * 2))
        .take(n)
        .collect()
}
