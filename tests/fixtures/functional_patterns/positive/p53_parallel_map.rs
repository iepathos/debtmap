use rayon::prelude::*;

fn parallel_process(items: Vec<i32>) -> Vec<i32> {
    items.par_iter()
        .map(|x| x * 2)
        .collect()
}
