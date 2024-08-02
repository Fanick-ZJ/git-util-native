#![deny(clippy::all)]
use std::{collections::HashMap, io, os::windows::process::CommandExt, process::{Command, Output}};
use napi::{Error as napiError, JsError};

mod structs;
#[macro_use]
extern crate napi_derive;

fn get_command_output(prog: &str, path: &str, args: &[&str]) -> io::Result<Output> {
    let mut cmd = Command::new(prog);
    args.iter().for_each(|arg| {
        cmd.arg(arg);
    });
    // 创建进程时，设置创建进程的标志，以隐藏窗口
    cmd.creation_flags(0x08000000);
    cmd.current_dir(path);
    cmd.output()
}

// 使用Result来处理可能会抛出异常的函数

#[napi]
/**
 * Check if git is installed
 */
pub fn has_git () -> bool {
    let output = get_command_output("git", "", &["--version"]).expect("failed to execute process");
    if output.status.success() {
        return true
    }

   return false
}

#[napi]
/**
 * Check if a path is a git repository
 * @param path path to the repository
 */
fn is_git_repository(path: String) -> bool {
    let output = get_command_output("git", &path, &["rev-parse", "--is-inside-work-tree"]);
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
 * @param path path to the repository
 */
fn get_branches(path: String) -> Result<Vec<String>, JsError> {
    let output = get_command_output("git", &path, &["branch", "--all"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let branches = stdout
                .lines()
                .map(|line| {
                    let tmp = line.trim_start_matches('*').trim().split(" ").into_iter().next().unwrap();
                    return tmp.to_string();
                }).collect();
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
 * @param path path to the repository
 */
fn is_commited (path: String) -> Result<bool, JsError> {
    let output = get_command_output("git", &path, &["status", "--porcelain"]);
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

#[napi]
/**
 * Get the current branch name
 * @param path path to the repository
 */
fn get_current_branch(path: String) -> Result<String, JsError> {
    let output = get_command_output("git", &path, &["rev-parse", "--abbdev-ref", "HEAD"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim().to_string())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get the remote of a branch
 * @param path path to the repository
 * @param branch branch name
 */
fn get_branch_in_remote (path: String, branch: String) -> Result<String, JsError> {
    let arg = "branch.".to_string() + &branch + ".remote";
    let output = get_command_output("git", &path, &["config", "--get", &arg]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim().to_string())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Check this repository has remote
 * @param path path to the repository
*/
fn has_remote (path: String) -> Result<bool, JsError> {
    let output = get_command_output("git", &path, &["remote", "show"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(!stdout.trim().is_empty())
        }
        Err(e) => {
           let err = napiError::from(e);
           Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get all remotes of a repository
 * @param path path to the repository
*/
fn get_remote (path: String) -> Result<Vec<String>, JsError> {
    let output = get_command_output("git", &path, &["remote", "show"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim().split("\n").map(|s| s.to_string()).collect())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get all tags of a repository
 * @param path path to the repository
*/
fn get_tags (path: String) -> Result<Vec<String>, JsError> {
    let output = get_command_output("git", &path, &["tag"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim().split("\n").map(|s| s.to_string()).collect())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

fn get_format_key_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("%H".to_string(), "hashL".to_string());
    map.insert("%h".to_string(), "hashS".to_string());
    map.insert("%T".to_string(), "treeL".to_string());
    map.insert("%t".to_string(), "treeS".to_string());
    map.insert("%P".to_string(), "parentHashL".to_string());
    map.insert("%p".to_string(), "parentHashS".to_string());
    map.insert("%an".to_string(), "authorName".to_string());
    map.insert("%ae".to_string(), "authorEmail".to_string());
    map.insert("%ad".to_string(), "date".to_string());
    map.insert("%ar".to_string(), "dateRelative".to_string());
    map.insert("%at".to_string(), "dateTimeStamp".to_string());
    map.insert("%cn".to_string(), "committerName".to_string());
    map.insert("%ce".to_string(), "committerEmail".to_string());
    map.insert("%cd".to_string(), "committerDate".to_string());
    map.insert("%cr".to_string(), "committerDateRelative".to_string());
    map.insert("%ct".to_string(), "committerDateTimeStamp".to_string());
    map.insert("%cs".to_string(), "committerDateYMD".to_string());
    map.insert("%d".to_string(), "refs".to_string());
    map.insert("%D".to_string(), "refsComma".to_string());
    map.insert("%s".to_string(), "message".to_string());
    map.insert("%b".to_string(), "body".to_string());
    map.insert("%B".to_string(), "bodyNoTrailingSlash".to_string());
    map.insert("%N".to_string(), "notes".to_string());

    return map;
}

#[napi]
/**
 * Get the commit log of a repository
 * You can use placeholders to get the commit log information.
 * The placeholders are:
 * | Placeholders | Description | key |
 * | ---- | ---- | ---- |
 * |%H    | commit hash| hashL |
 * |%h    | abbreviated commit hash| hashS |
 * |%T    | tree hash | treeL |
 * |%t    | abbreviated tree hash| treeS |
 * |%P    | parent hashes | parentHashL |
 * |%p    | abbreviated parent hashes | parentHashS |
 * |%an   | author name | authorName |
 * |%ae   | author email | authorEmail |
 * |%ad   | author date  | date |
 * |%ar   | author date, relative with now | dateRelative |
 * |%at   | author date, unix timestamp | dateTimeStamp |
 * |%ai   | author date, ISO 8601 format | dateIso |
 * |%as   | author date, short format (YYYY-MM-DD) | dateYMD |
 * |%ah   | author date, human-readable format | dateHuman |
 * |%cn   | committer name | committerName |
 * |%ce   | committer email | committerEmail |
 * |%cd   | committer date | committerDate |
 * |%cr   | committer date, relative with now | committerDateRelative |
 * |%ct   | committer date, unix timestamp | committerDateTimeStamp |
 * |%cs   | committer date, short format (YYYY-MM-DD) | committerDateYMD |
 * |%ch   | committer date, human-readable format | committerDateHuman |
 * |%d    | ref names | refs |
 * |%D    | ref names, separated by commas | refsComma |
 * |%s    | subject | message |
 * |%b    | body | body |
 * |%B    | body, without trailing slash | bodyNoTrailingSlash |
 * |%N    | commit notes | notes |
 */
fn get_commit_log_format(path: String, branch: String, placeholders: Vec<String>) -> Result<Vec<HashMap<String, String>>, JsError> {
    let interval = "<<INTERVAL>>";
    let commit_end = "<<COMMIT_END>>";
    let mut format = String::from("--pretty=format:");
    for key in placeholders.iter(){
        format = format + &key + interval;
    }
    format = format.trim_end_matches(interval).to_string() + commit_end;
    let key_map = get_format_key_map();
    let output = get_command_output("git", &path, &["log", &branch, &format]);
    let mut res = Vec::new();
    match output{
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.trim().trim_end_matches(&commit_end).split(&commit_end) {
                let datas = line.split(interval).collect::<Vec<_>>();
                let mut map = HashMap::<String, String>::new();
                for i in 0..placeholders.len(){
                    let key = placeholders[i].to_string();
                    let value = datas[i].to_string();
                    let key = key_map.get(&key).unwrap();
                    map.insert(key.to_string(), value.to_string());
                }
                res.push(map);
            }
            Ok(res)
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}