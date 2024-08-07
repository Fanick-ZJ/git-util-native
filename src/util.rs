use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};


pub fn get_basename(path: &str) -> Option<String> {
    let path = Path::new(path);
    path.file_name().and_then(|f| f.to_str()).map(String::from)
}

pub fn get_directory_path(path: &str) -> Option<String> {
    let path = Path::new(path);
    path.parent().and_then(|f| f.to_str()).map(String::from)
}

pub fn get_current_time() -> u128 {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    return time;
}