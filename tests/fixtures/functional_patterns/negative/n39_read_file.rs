use std::fs;

fn read_content(path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}
