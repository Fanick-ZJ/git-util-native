#![deny(clippy::all)]
use std::process::Command;
use napi::{Error as napiError, JsError};
#[macro_use]
extern crate napi_derive;

// 使用Result来处理可能会抛出异常的函数

#[napi]
/**
 * Check if git is installed
 */
pub fn has_git () -> bool {
    let output = std::process::Command::new("git")
        .arg("--version")
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        return true
    }

   return false
}

#[napi]
/**
 * Check if a path is a git repository
 */
fn is_git_repository(path: String) -> bool {
    let output = Command::new("git")
        .current_dir(path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout == "true"
        }
        Err(_) => false,
    }
}

#[napi]
/**
 * Get all branches in a git repository
 */
fn get_branches(path: String) -> Result<Vec<String>, JsError> {
    let output = Command::new("git")
        .current_dir(path)
        .arg("branch")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let branches: Vec<String> = stdout
                .lines()
                .map(|line| line.trim_start_matches('*').trim().to_string())
                .collect();
            Ok(branches)
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        },
    }
}