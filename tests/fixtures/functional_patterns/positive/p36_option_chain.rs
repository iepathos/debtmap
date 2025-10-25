fn process_optional(opt: Option<i32>) -> Option<i32> {
    opt.map(|x| x * 2)
       .filter(|&x| x > 0)
       .map(|x| x + 1)
}
