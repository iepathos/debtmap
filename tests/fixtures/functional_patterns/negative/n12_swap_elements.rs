fn bubble_sort(mut items: Vec<i32>) -> Vec<i32> {
    for i in 0..items.len() {
        for j in 0..items.len() - 1 {
            if items[j] > items[j + 1] {
                items.swap(j, j + 1);
            }
        }
    }
    items
}
