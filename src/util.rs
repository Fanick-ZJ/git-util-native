use std::path::Path;


pub fn get_basename(path: &str) -> Option<String> {
    let path = Path::new(path);
    path.file_name().and_then(|f| f.to_str()).map(String::from)
}