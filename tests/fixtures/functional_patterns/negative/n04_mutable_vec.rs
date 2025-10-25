fn mutate_in_place(items: &mut Vec<i32>) {
    for i in 0..items.len() {
        items[i] *= 2;
    }
}
