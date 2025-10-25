fn merge_with_filter(mut base: Vec<i32>, new: Vec<i32>) -> Vec<i32> {
    base.extend(new.into_iter().filter(|&x| x > 0));
    base
}
