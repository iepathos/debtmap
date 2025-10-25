fn complex_mutation(mut a: Vec<i32>, mut b: Vec<i32>) -> Vec<i32> {
    for item in b.drain(..) {
        a.push(item * 2);
    }
    a.sort();
    a
}
