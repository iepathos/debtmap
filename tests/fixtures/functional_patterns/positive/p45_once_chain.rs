fn prepend_value(items: Vec<i32>, val: i32) -> Vec<i32> {
    std::iter::once(val)
        .chain(items.into_iter())
        .collect()
}
