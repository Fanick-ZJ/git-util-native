#![deny(clippy::all)]
use std::{error::Error, process::Command};
use err::{build_git_error, CustomerGitError};
use napi::{Error as napiError, JsError};

mod err;
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
fn _is_git_repository(path: &str) -> Result<bool, Box<CustomerGitError>> {
    if !std::path::Path::new(path).exists() {
        let err = build_git_error(path, "Is not Exist");
        return Err(Box::new(err));
    }
    let output = Command::new("git")
        .current_dir(path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout == "true")
        }
        Err(e) => {
            return Err(Box::new(build_git_error(&path, &e.to_string())))
        }
    }
}

#[napi]
/**
 * Get all branches in a git repository
 */
fn get_branches(path: String) -> Result<Vec<String>, JsError> {
    // match _is_git_repository(&path) {
    //     Ok(true) => {}
    //     Ok(false) => {
    //         let err = napiError::from(Box::new(build_git_error(&path, "Is not a git repository")));
    //     }
    //     Err(err) => {
    //         return Err(JsError::from_reason(err.to_string()));
    //     }
    // }

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

#[napi]
/**
 * Check if a git repository is commited
 */
fn is_commited (path: String) -> Result<bool, JsError> {
    let output = Command::new("git")
        .current_dir(path)
        .arg("status")
        .arg("--porcelain")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.is_empty())
        }
       Err(e) => {
          let err = napiError::from(e);
          Err(JsError::from(err))
       }
    }
}