fn nested_iteration(matrix: Vec<Vec<i32>>) -> i32 {
    let mut sum = 0;
    for row in matrix {
        for val in row {
            sum += val;
        }
    }
    sum
}
