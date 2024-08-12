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

pub fn build_commit_range(start: &str, end: &str) -> String {
    let commit_range = if start.is_empty() && end.is_empty(){
        String::from("HEAD")
    } else if start.is_empty() && !end.is_empty(){
        format!("{}", end)
    } else if !start.is_empty() && end.is_empty(){
        format!("{}^..HEAD", start)
    } else {
        format!("{}^..{}", start, end)
    };
    return commit_range;
}