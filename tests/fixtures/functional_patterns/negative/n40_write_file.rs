use std::fs;

fn save_content(path: &str, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content)
}
