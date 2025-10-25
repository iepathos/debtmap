use rayon::prelude::*;

fn parallel_filter(items: Vec<i32>) -> Vec<i32> {
    items.into_par_iter()
        .filter(|&x| x > 0)
        .collect()
}
