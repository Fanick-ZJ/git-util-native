#![deny(clippy::all)]
use std::{collections::{HashMap, HashSet}, io, os::windows::process::CommandExt, process::{Command, Output}};
use napi::{Error as napiError, JsError};
use serde_json::Error;
use structs::{Author, AuthorStatDailyContribute, Branch, BranchCreatedInfo, BranchStatDailyContribute, Remote, RepoFileInfo, RepositoryFull, RepositorySimple, StatDailyContribute};
use util::get_basename;


mod structs;
mod util;
#[macro_use]
extern crate napi_derive;

static PARAM_INTERVAL: &str = "<<PARAM_INTERVAL>>";
static COMMIT_INETRVAL: &str = "<<COMMIT_INETRVAL>>";

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
fn get_current_branch(path: String) -> Result<Branch, JsError> {
    let output = get_command_output("git", &path, &["rev-parse", "--abbrev-ref", "HEAD"]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let author = get_branch_authors(path.to_string(), stdout.to_string())?;
            let created_info = get_branch_create_info(path.to_string(), stdout.to_string())?;
            Ok(Branch {
                name: stdout.to_string(),
                created: created_info,
                authors: author,
            })
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
fn get_remote (path: String) -> Result<Vec<Remote>, JsError> {
    let output = get_command_output("git", &path, &["remote", "-v"]);
    match output {
        Ok(output) => {
            let mut remotes = HashMap::<String, Remote>::new();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines = stdout.trim().split("\n").collect::<Vec<&str>>();
            for line in lines {
                let parts = line.trim().split_whitespace().collect::<Vec<&str>>();
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let operate = parts[2].trim_start_matches("(").trim_end_matches(")").to_string();
                let remote = remotes.get_mut(&name);
                if let Some(remote) = remote {
                    remote.operate.push(operate);
                } else {
                    remotes.insert(name.to_string(), Remote {
                        name: name.to_string(),
                        url,
                        operate: vec![operate],
                    });
                }
            }
            Ok(remotes.into_values().collect())
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
    let mut format = String::from("--pretty=format:");
    for key in placeholders.iter(){
        format = format + &key + PARAM_INTERVAL;
    }
    format = format.trim_end_matches(PARAM_INTERVAL).to_string() + COMMIT_INETRVAL;
    let key_map = get_format_key_map();
    let output = get_command_output("git", &path, &["log", &branch, &format]);
    let mut res = Vec::new();
    match output{
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.trim().trim_end_matches(&COMMIT_INETRVAL).split(&COMMIT_INETRVAL) {
                let datas = line.split(PARAM_INTERVAL).collect::<Vec<_>>();
                let mut map = HashMap::<String, String>::new();
                for i in 0..placeholders.len(){
                    let key = placeholders[i].to_string();
                    let value = datas[i].trim().to_string();
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

#[napi]
/**
 * Get the authors of a repository
 * @param path path to the repository
 * @param branch branch to get the authors from
*/
fn get_branch_authors (path: String, branch: String) ->Result<Vec<Author>, JsError> {
    let output = get_command_output("git", &path, &["shortlog", &branch, "-sne"]);
    match output {
        Ok(output) => {
            let mut authors = Vec::<Author>::new();
            let lines = String::from_utf8_lossy(&output.stdout);
            for line in lines.trim().split("\n") {
                let keys = line.split_ascii_whitespace().collect::<Vec<_>>();
                let author_name = keys[1].to_string();
                let author_email = keys[2].to_string();
                authors.push(Author {
                    name: author_name,
                    email: author_email,
                });
            }
            Ok(authors)
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get the authors of a repository
 * @param path path to the repository
*/
fn get_all_authors (path: String) -> Result<Vec<Author>, JsError> {
    let placeholders = vec![String::from("%an"), String::from("%ae")];
    let output = get_command_output("git", &path, &["shortlog", "-sne"]);
    match output {
        Ok(output) => {
            let mut authors = HashSet::<Author>::new();
            let lines = String::from_utf8_lossy(&output.stdout);
            for line in lines.trim().split("\n") {
                let keys = line.split_ascii_whitespace().collect::<Vec<_>>();
                let author_name = keys[1].to_string();
                let author_email = keys[2].to_string();
                authors.insert(Author {
                    name: author_name,
                    email: author_email,
                });
            }
            Ok(authors.into_iter().collect::<Vec<_>>())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get the branch creation info of a repository
 * @param path path to the repository
 * @param branch branch to get the branch creation info from
*/
fn get_branch_create_info (path: String, branch: String) -> Result<BranchCreatedInfo, JsError> {
    let format = "--pretty=format:".to_string() + "%an" + PARAM_INTERVAL + "%ae" + PARAM_INTERVAL + "%at" + PARAM_INTERVAL + "%H";
    let output = get_command_output("git", &path, &["log", &branch, "--reverse", "--max-parents=0", &format]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let keys = stdout.trim().split(PARAM_INTERVAL).collect::<Vec<_>>();
            let author_name = keys[0].to_string();
            let author_email = keys[1].to_string();
            let hash = keys[3].to_string();
            let time = keys[2].to_string();
            let author = Author {
                name: author_name,
                email: author_email,
            };
            Ok(BranchCreatedInfo {
                name: branch,
                time: time,
                author: author,
                hash
            })
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }


}

#[napi]
/**
 * Get the branch creation info of a repository
 * @param path path to the repository
 * @param branchs branchs to get the branch creation info from
*/
fn get_branchs_create_info (path: String, branchs: Vec<String>) -> Result<Vec<BranchCreatedInfo>, JsError> {
    let mut res = Vec::<BranchCreatedInfo>::new();
    for branch in branchs.iter() {
        let info = get_branch_create_info(path.to_string(), branch.to_string());
        match info {
            Ok(info) => {
                res.push(info);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(res)   
}

#[napi]
/**
 * Get the repository info of a repository
 * @param path path to the repository
*/
fn get_repository_info_full (path: String) -> Result<RepositoryFull, JsError> {
    let branches = get_branches(path.to_string())?;
    let authors = get_all_authors(path.to_string())?;
    let current_branch = get_current_branch(path.to_string())?;
    let mut branches_arr = Vec::<Branch>::new();
    for branch in branches.iter() {
        // get branch info
        let branch_name = branch.to_string();
        let branch_info = get_branch_create_info(path.to_string(), branch_name.to_string())?;
        let branch_authors = get_branch_authors(path.to_string(), branch_name.to_string())?;
        let branch = Branch {
            name: branch_name.to_string(),
            created: branch_info,
            authors: branch_authors.clone(),
        };
        branches_arr.push(branch);
    };
    
    // get repository name
    let repo_name = util::get_basename(&path);
    let name;
    match repo_name {
        Some(_name) => {
            name = _name;
        }
        None => {
            name= String::from("");
        }
    };
    // get remote 
    let remote = get_remote(path.to_string())?;
    Ok(RepositoryFull {
        current_branch: current_branch.clone(),
        branches: branches_arr.iter().map(| item | (*item).clone()).collect::<Vec<Branch>>(),
        authors: authors.iter().map(| item | (*item).clone()).collect::<Vec<Author>>(),
        name: name,
        remote: remote,
        path: path,
    })
}

#[napi]
/**
 * Get the repository info in a simple way
 * @param path path to the repository
 */
fn get_repository_info_simple (path: String) -> Result<RepositorySimple, JsError> {
    let branches = get_branches(path.to_string())?;
    let current_branch = get_current_branch(path.to_string())?;
    let authors = get_all_authors(path.to_string())?;
    let mut branches_arr = Vec::<String>::new();
    let remote = get_remote(path.to_string())?;
    for branch in branches.iter() {
        branches_arr.push(branch.to_string());
    }
    Ok(RepositorySimple {
        name: util::get_basename(&path).unwrap(),
        branches: branches_arr,
        current_branch: current_branch.name,
        path: path,
        authors,
        remote
    })
}

#[napi]
/**
 * Get the statistic of daily contribute in a branch
 */
fn get_contribute_stat (path: String, branch: String) -> Result<BranchStatDailyContribute, JsError> {
    let format = "--pretty=format:".to_string()+ COMMIT_INETRVAL + "%an" + PARAM_INTERVAL + "%ae" + PARAM_INTERVAL + "%at";
    let branch_create_info = get_branch_create_info(path.to_string(), branch.to_string())?;
    let start_flag = format!("{}..HEAD", branch_create_info.hash);
    let output = get_command_output("git", &path, &["log", &branch, &start_flag, "--shortstat", &format, "--reverse"]);
    match output {
        Ok(output) => {
            let mut authors_stat = HashMap::<String, AuthorStatDailyContribute>::new();
            let mut total_stat = StatDailyContribute {
                commit_count: 0,
                data_list: Vec::<String>::new(),
                insertion: Vec::<i32>::new(),
                deletions: Vec::<i32>::new(),
                change_files: Vec::<i32>::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            let commits = stdout.trim().split(COMMIT_INETRVAL).filter(| line | line.to_string() != "").collect::<Vec<_>>();
            // parse commits
            for i in 0..commits.len() {
                let commit = commits[i];
                let lines = commit.trim_end_matches("\n").split("\n").collect::<Vec<_>>();
                if lines.len() != 2 {continue;}
                let auth_info = lines[0].split(PARAM_INTERVAL).collect::<Vec<_>>();
                let change_info = lines[1].trim().split(", ").collect::<Vec<_>>();
                // println!("======================\n{}\n{}\n============================", auth_info.join("|"), change_info.join("|"));
                // the first change is number of change files, if has insert , 
                // the second is number of insertions, and the third is number of deletions
                // if not insert, that the second is number of deletions
                let change1_info = change_info[0].split(" ").collect::<Vec<_>>();
                let change2_info = change_info[1].split(" ").collect::<Vec<_>>();
                let name = auth_info[0].to_string();
                let email = auth_info[1].to_string();
                let date = auth_info[2].to_string();
                // if this author has contained
                if authors_stat.contains_key(&name) {
                    let author = authors_stat.get_mut(&name).unwrap();
                    author.stat.commit_count += 1;
                    let len = author.stat.data_list.len();
                    // if one day has multiple commits
                    if author.stat.data_list[len - 1] == date { 
                        author.stat.change_files[len - 1] = change1_info[0].parse::<i32>().unwrap() + author.stat.change_files[len - 1];
                        if change2_info[1].starts_with("insertion") {
                            author.stat.insertion[len - 1] = change2_info[0].parse::<i32>().unwrap() + author.stat.insertion[len - 1];
                        }
                        else {
                            author.stat.deletions[len - 1] = change2_info[0].parse::<i32>().unwrap() + author.stat.deletions[len - 1];
                        }
                        if change_info.len() > 2 {
                            author.stat.deletions[len - 1] = change2_info[0].parse::<i32>().unwrap() + author.stat.deletions[len - 1];
                        }
                    } else {
                        // new day and first commit
                        author.stat.data_list.push(date.to_string());
                        if change2_info[1].starts_with("insertion") {
                            author.stat.insertion.push(change2_info[0].parse::<i32>().unwrap());
                            if change_info.len() == 2 { total_stat.deletions.push(0); }
                        }
                        else {
                            author.stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                            author.stat.insertion.push(0);
                        }
                        if change_info.len() > 2 {
                            author.stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                        }
                    }
                } else {
                    // new author
                    let mut author = AuthorStatDailyContribute {
                        author: Author {
                            name: name.to_string(),
                            email: email,
                        },
                        stat: StatDailyContribute {
                            commit_count: 1,
                            data_list: Vec::<String>::new(),
                            insertion: Vec::<i32>::new(),
                            deletions: Vec::<i32>::new(),
                            change_files: Vec::<i32>::new(),
                        }
                    };
                    author.stat.data_list.push(date.to_string());
                    author.stat.change_files.push(change1_info[0].parse::<i32>().unwrap());
                    if change2_info[1].starts_with("insertion") {
                        author.stat.insertion.push(change2_info[0].parse::<i32>().unwrap());
                        if change_info.len() == 2 { author.stat.deletions.push(0); }
                    }
                    else {
                        author.stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                        author.stat.insertion.push(0);
                    }
                    if change_info.len() > 2 {
                        author.stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                    }
                    authors_stat.insert(name, author);
                }
                // total stat
                total_stat.commit_count += 1;
                let len = total_stat.data_list.len();
                if len > 0 && total_stat.data_list[len - 1] == date {
                    total_stat.change_files[len - 1] = change1_info[0].parse::<i32>().unwrap() + total_stat.change_files[len - 1];
                    if change2_info[1].starts_with("insertion") {
                        total_stat.insertion[len - 1] = change2_info[0].parse::<i32>().unwrap() + total_stat.insertion[len - 1];
                    }
                    else {
                        total_stat.deletions[len - 1] = change2_info[0].parse::<i32>().unwrap() + total_stat.deletions[len - 1];
                    }
                    if change_info.len() > 2 {
                        total_stat.deletions[len - 1] = change2_info[0].parse::<i32>().unwrap() + total_stat.deletions[len - 1];
                    }
                } else {
                    // new day and first commit
                    total_stat.data_list.push(date.to_string());
                    total_stat.change_files.push(change1_info[0].parse::<i32>().unwrap());
                    if change2_info[1].starts_with("insertion") {
                        total_stat.insertion.push(change2_info[0].parse::<i32>().unwrap());
                        if change_info.len() == 2 { total_stat.deletions.push(0); }
                    }
                    else {
                        total_stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                        total_stat.insertion.push(0);
                    }
                    if change_info.len() > 2 {
                        total_stat.deletions.push(change2_info[0].parse::<i32>().unwrap());
                    }
                    // println!("{} {} {} {} {}",change_info.len(),  total_stat.data_list.len(), total_stat.insertion.len(), total_stat.deletions.len(), total_stat.change_files.len())
                }
            }
            Ok(BranchStatDailyContribute {
                branch: branch,
                total_stat: total_stat,
                authors_stat: authors_stat.into_values().collect::<Vec<AuthorStatDailyContribute>>(),
            })
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }

}

/**
 * Insert the file info list
 */
fn insert_file_to_tree(file_list: &mut Vec<RepoFileInfo>, object_mode: &str, object_type: &str, object_name: &str, object_size: &str, object_path: &str) {
    let file_tree = object_path.split("/").collect::<Vec<&str>>();
    let mut tmp_file_list = file_list;
    // println!("{}", file_tree.len());
    for i in 0..file_tree.len() {
        let index = tmp_file_list.iter().position(|t| t.name == file_tree[i] && t.is_dir);
        match index {
            Some( index ) => {
                // println!("{}", index);
                tmp_file_list = &mut tmp_file_list[index].children;
            }
            None => {
                let is_last = i == file_tree.len() - 1;
                let is_dir = object_mode.starts_with("040000") || !is_last;
                // println!("{} {} {} {} {} {}",file_tree.len() - 1, i, file_tree[i], is_dir, is_last, object_path);
                let dir = if i != 0 { file_tree[0..i].join("/") } else {"./".to_string()};
                let file = RepoFileInfo {
                    name: file_tree[i].to_string(),
                    dir,
                    object_mode: if is_last {object_mode.to_string()} else {"".to_string()},
                    object_type: if is_last {object_type.to_string()} else {"".to_string()},
                    object_name: if is_last {object_name.to_string()} else {"".to_string()},
                    object_size: if is_last {object_size.to_string()} else {"".to_string()},
                    is_dir,
                    children: Vec::<RepoFileInfo>::new(),
                };
                tmp_file_list.push(file);
                // println!("{:?}", tmp_file_list);
                if is_dir {
                    tmp_file_list = tmp_file_list.last_mut().unwrap().children.as_mut();
                }
            }
        }
        
    }
}

/**
 * From the file info list, generate the file tree
 */
fn file_info_list_to_tree (file_info_list: Vec<&str>) -> Vec<RepoFileInfo> {
    let mut file_list: Vec<RepoFileInfo> = Vec::new();
    for line in file_info_list {
        let file_info = line.split(PARAM_INTERVAL).collect::<Vec<&str>>();
        if file_info.len() == 5 {
            let object_mode = file_info[0];
            let object_type = file_info[1];
            let object_size = file_info[2].trim();
            let object_name = file_info[3];
            let object_path = file_info[4];
            // objectMode Code:
            // 040000: Directory
            // 100644: Regular non-executable file
            // 100664: Regular non-executable group-writeable file
            // 100755: Regular executable file
            // 120000: Symbolic link
            // 160000: Gitlink
            let file_tree = object_path.split("/").collect::<Vec<&str>>();
            if file_tree.len() > 1 {
                insert_file_to_tree(&mut file_list, object_mode, object_type, object_name, object_size, object_path)
            } else { 
                let is_dir = object_mode.starts_with("040000");
                let file = RepoFileInfo {
                    name: object_path.to_string(),
                    dir: "./".to_string(),
                    object_mode: object_mode.to_string(),
                    object_type: object_type.to_string(),
                    object_name: object_name.to_string(),
                    object_size: object_size.to_string(),
                    is_dir,
                    children: Vec::<RepoFileInfo>::new(),
                };
                file_list.push(file);
            }
       }
    }
    return file_list
}

#[napi]
/**
 * Get the file list of a repository
 */
fn get_repo_file_list (path: String, branch_or_hash: String) -> Result<Vec<RepoFileInfo>, JsError> {
    let format = format!("--format=\"%(objectmode){}%(objecttype){}%(objectsize:padded){}%(objectname){}%(path)\"", PARAM_INTERVAL, PARAM_INTERVAL, PARAM_INTERVAL, PARAM_INTERVAL);
    let output = get_command_output("git", &path, &["ls-tree", "-r", &branch_or_hash, &format]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines = stdout.trim().split("\n").collect::<Vec<&str>>();
            let file_list = file_info_list_to_tree(lines);
            Ok(file_list)

        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}