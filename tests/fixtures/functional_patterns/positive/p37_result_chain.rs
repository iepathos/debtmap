fn process_fallible(res: Result<i32, String>) -> Result<i32, String> {
    res.map(|x| x * 2)
       .and_then(|x| if x > 0 { Ok(x) } else { Err("Negative".to_string()) })
}
