fn get_page(items: Vec<i32>, page: usize, size: usize) -> Vec<i32> {
    items.into_iter()
        .skip(page * size)
        .take(size)
        .collect()
}
