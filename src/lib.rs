#![deny(clippy::all)]
use regex::Regex;
use std::{collections::{HashMap, HashSet}, env::VarError, error::Error, fmt::format, io, os::windows::process::CommandExt, process::{Command, Output}};
use napi::{Error as napiError, JsError};
use structs::{Author, AuthorStatDailyContribute, Branch, BranchCreatedInfo, BranchStatDailyContribute, FileDiffContext, FileLineChangeStat, FileStatus, FileStatusReport, FileStatusType, Remote, RepoFileInfo, RepositoryFull, RepositorySimple, StatDailyContribute};
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

fn log_shortstat_parse (status: &str) -> Result<(i32, i32, i32), String> {
    let re = Regex::new(r"(?<changes>\d+) files? changed(?:, (?<insertions>\d+) insertions?\(\+\))?(?:, (?<deletions>\d+) deletions?\(-\))?").unwrap();
    let Some(captures) = re.captures(status) else {
        return Err("No match found!".to_string())
    };
    let mut insertions = 0;
    let mut changes = 0;
    let mut deletions = 0;
    if let Some(changes_str) = captures.name("changes") {
        changes = changes_str.as_str().parse::<i32>().unwrap();
    }
    if let Some(insertions_str) = captures.name("insertions") {
        insertions = insertions_str.as_str().parse::<i32>().unwrap();
    }
    if let Some(deletions_str) = captures.name("deletions") {
        deletions = deletions_str.as_str().parse::<i32>().unwrap();
    }
    Ok((changes, insertions, deletions))
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
                // parse shortstat
                let Ok ((changes, insertions, deletions)) = log_shortstat_parse(lines[1]) else {
                    let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("")));
                    return Err(JsError::from(err))
                };
                // println!("======================\n{}\n{}\n============================", auth_info.join("|"), change_info.join("|"));
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
                        author.stat.change_files[len - 1] = changes;
                        author.stat.insertion[len - 1] = insertions;
                        author.stat.deletions[len - 1] = deletions;
                    } else {
                        // new day and first commit
                        author.stat.data_list.push(date.to_string());
                        author.stat.insertion.push(insertions);
                        author.stat.deletions.push(deletions);
                        author.stat.change_files.push(changes);
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
                    author.stat.insertion.push(insertions);
                    author.stat.deletions.push(deletions);
                    author.stat.change_files.push(changes);
                    authors_stat.insert(name, author);
                }
                // total stat
                total_stat.commit_count += 1;
                let len = total_stat.data_list.len();
                if len > 0 && total_stat.data_list[len - 1] == date {
                    total_stat.change_files[len - 1] = changes;
                    total_stat.insertion[len - 1] = insertions;
                    total_stat.deletions[len - 1] = deletions;
                } else {
                    // new day and first commit
                    total_stat.data_list.push(date.to_string());
                    total_stat.data_list.push(date.to_string());
                    total_stat.insertion.push(insertions);
                    total_stat.deletions.push(deletions);
                    total_stat.change_files.push(changes);
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

#[napi]
/**
 * Get the file status of a commit
 */
fn get_commit_file_status (path: String, hash: String) -> Result<FileStatusReport, JsError> {
    let format = format!("--format=%H{}%s{}%an{}%ae{}%at", PARAM_INTERVAL, PARAM_INTERVAL, PARAM_INTERVAL, PARAM_INTERVAL);
    let output = get_command_output("git", &path, &["show", &hash, "--name-status", "--oneline", &format]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines = stdout.trim().split("\n").filter(|t| *t != "").collect::<Vec<&str>>();
            let commit_info = lines[0].trim().split(PARAM_INTERVAL).collect::<Vec<&str>>();
            let commit_hash = commit_info[0];
            let commit_message = commit_info[1];
            let commit_author = commit_info[2];
            let commit_author_email = commit_info[3];
            let commit_time = commit_info[4];
            let file_status = lines[1..].iter().map(|line| {
                let params = line.split("\t").collect::<Vec<&str>>();
                let file_path = params[1].to_string();
                let mut message = "".to_string();
                let status_flag = params[0][0..1].to_string();
                let status = match status_flag.as_str() {
                    "A" => FileStatusType::Added,
                    "D" => FileStatusType::Deleted,
                    "M" => FileStatusType::Modified,
                    "R" => {
                        if params.len() == 3 {
                            message = params[1].to_string() + " => " + params[2];
                        }
                        FileStatusType::Renamed
                    },
                    "C" => FileStatusType::Copied,
                    "U" => FileStatusType::Updated,
                    _ => FileStatusType::Unknown,
                };
                FileStatus {
                    path: file_path,
                    status,
                    message,
                }
            }).collect::<Vec<FileStatus>>();
            let file_status_report = FileStatusReport {
                title: commit_message.to_string(),
                hash: commit_hash.to_string(),
                status: file_status,
                time: commit_time.to_string(),
                author: Author {
                    name: commit_author.to_string(),
                    email: commit_author_email.to_string(),
                }
            };
            Ok(file_status_report)
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

fn parse_file_status (status_flag: &str) -> FileStatusType {
    match status_flag {
        "A" => FileStatusType::Added,
        "D" => FileStatusType::Deleted,
        "M" => FileStatusType::Modified,
        "R" => FileStatusType::Renamed,
        "C" => FileStatusType::Copied,
        "U" => FileStatusType::Updated,
        _ =>FileStatusType::Unknown,
    }
}

/**
 * Get the file list of a repository
 */
fn get_file_between_commit_status(path: String, commit_hash1: String, file_path: String) -> Result<(FileStatusType, String), String> {
    let output: Result<Output, io::Error> = get_command_output("git", &path, &["show", &commit_hash1, "--name-status",  "--format=", "--", &file_path]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines = stdout.trim().split_ascii_whitespace().collect::<Vec<&str>>();
            if lines.len() > 0 {
                let status_flag = lines[0][0..1].to_string();
                let mut message = "".to_string();
                let file_status = parse_file_status(&status_flag);
                if file_status == FileStatusType::Renamed {
                    if lines.len() > 1 {
                        message = lines[1].to_string() + " => " + lines[2];
                    }
                }
                Ok((file_status, message))
            }
            else {
                Err("No status found".to_string())
            }
        }
        Err(e) => {
            Err(e.source().unwrap().to_string())
        }
    }
}

#[napi]
/**
 * Get the file change statistic between two commits
 * @param path The path of the repository
 * @param commit_hash1 The commit hash of the first commit
 * @param commit_hash2 The commit hash of the second commit
 * @param file_path The path of the file
 */
fn get_file_modify_stat_between_commit(path: String, commit_hash1: String, commit_hash2: String, file_path: String) -> Result<FileLineChangeStat, JsError> {
    let commit_range = format!("{}...{}", commit_hash1, commit_hash2);
    let output = get_command_output("git", &path, &["diff", &commit_range , "--shortstat", "--", &file_path]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match log_shortstat_parse(&stdout) {
                Ok((_, addition, deletion)) => {
                    Ok(FileLineChangeStat {
                        addition,
                        deletion
                    })
                }
                Err(_) => {
                    let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file change status:\nRepository path: {}\ncommit hash1: {}\ncommit hash2: {}", path, commit_hash1, commit_hash2)));
                    Err(JsError::from(err))
                }
            }
        }
        Err(e) => {
            let err = napiError::from(e);
            return Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get the files change status between two commits
 * @param path The path of the repository
 * @param commit_hash1 The commit hash of the first commit
 * @param commit_hash2 The commit hash of the second commit
 */
fn get_files_status_between_commit (path: String, commit_hash1: String, commit_hash2: String) -> Result<Vec<FileStatus>, JsError> {
    let output = get_command_output("git", &path, &["diff", "--name-status", &commit_hash1, &commit_hash2]);
    let mut file_status = Vec::<FileStatus>::new();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines = stdout.trim().split("\n").collect::<Vec<&str>>();
            for line in lines.iter() {
                let params = line.split_ascii_whitespace().collect::<Vec<&str>>();
                let flag = params[0][0..1].to_string();
                let file_staus = parse_file_status(&flag);
                let mut message = "".to_string();
                if file_staus == FileStatusType::Renamed {
                    if params.len() > 2 {
                        message = params[1].to_string() + " => " + params[2];
                    }
                }
                file_status.push(FileStatus {
                    path: params[1].to_string(),
                    status: file_staus,
                    message,
                });
            }
            Ok(file_status)
        }
        Err(e) => {
            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get commit status:\nRepository path: {}\ncommit hash1: {}\ncommit hash2: {}", path, commit_hash1, commit_hash2)));
            return Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * Get the file diff context of a file
 * @param repo: the path of the repository
 * @param commit_hash1: the hash of the first commit
 * @param commit_hash2: the hash of the second commit
 * @param file_path: the path of the file
 */
fn diff_file_context (repo: String, commit_hash1: String, commit_hash2: String, file_path: String) -> Result<FileDiffContext, JsError> {
    // 先用从 git show hash1 hash2 --name-status --format="" file_path 来获取文件在两个提交见的状态，是需改还是删除还是重命名等等
    // 如果是文件中的修改，则调用 git diff --shortstat hash1 hash2 -- file_path 来记录文件中修改的数量，二进制文件不需要做，只需要提示为二进制文件即可
    //      如果是重命名、删除的话，就不用做，提供说明
    // 如果是文件中修改的话，使用 git cat-file -p hash:path 来获取文件内容
    let commit_status = get_file_between_commit_status(repo.to_string(), commit_hash2.to_string(), file_path.to_string());
    match commit_status {
        Ok(commit_status) => {
            let (status, message) = commit_status;
            let mut context1 = "".to_string();
            let mut context2 = "".to_string();
            match status {
                // 添加
                FileStatusType::Added =>{
                    let output = get_command_output("git", &repo, &["cat-file", "-p", &format!("{}:{}", commit_hash1, file_path)]);
                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            context1 = stdout.to_string();
                            Ok(FileDiffContext {
                                commit_hash1: commit_hash1.to_string(),
                                commit_hash2: commit_hash2.to_string(),
                                file_path: file_path.to_string(),
                                change_stat: FileLineChangeStat {
                                    addition: stdout.trim().lines().count() as i32,
                                    deletion: 0,
                                },
                                context1,
                                context2,
                                file_status: status,
                            })
                        }
                        Err(e) => {
                            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_path, commit_hash1)));
                            Err(JsError::from(err))
                        }
                    }
                } 
                // 删除
                FileStatusType::Deleted => {
                    let output = get_command_output("git", &repo, &["cat-file", "-p", &format!("{}:{}", commit_hash1, file_path)]);
                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            context1 = stdout.to_string();
                            Ok(FileDiffContext {
                                commit_hash1: commit_hash1.to_string(),
                                commit_hash2: commit_hash2.to_string(),
                                file_path: file_path.to_string(),
                                change_stat: FileLineChangeStat {
                                    addition: 0,
                                    deletion: stdout.trim().lines().count() as i32,
                                },
                                context1,
                                context2,
                                file_status: status,
                            })
                        }
                        Err(e) => {
                            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_path, commit_hash2)));
                            Err(JsError::from(err))
                        }
                    }

                }
                // 修改
                FileStatusType::Modified => {
                    // 获取修改的数量
                    let output = get_command_output("git", &repo, &["diff", "--shortstat", &commit_hash1, &commit_hash2, "--", &file_path]);
                    let mut addition = 0;
                    let mut deletion = 0;
                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let lines = stdout.trim().split(", ").collect::<Vec<&str>>();
                            let change_info1 = lines[1].split(" ").collect::<Vec<&str>>();
                            if lines.len() > 2 {
                                if change_info1[1].starts_with("insertion") {
                                    addition = change_info1[0].parse::<i32>().unwrap();
                                    let change_info2 = lines[2].split(" ").collect::<Vec<&str>>();
                                    deletion = change_info2[0].parse::<i32>().unwrap();
                                }
                            } else {
                                if change_info1[1].starts_with("insertion") {
                                    addition = change_info1[0].parse::<i32>().unwrap();
                                } else {
                                    deletion = change_info1[0].parse::<i32>().unwrap();
                                }
                            }
                        }
                        Err(e) => {
                            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file diff:\nfile path: {}\ncommit hash1: {}\ncommit hash2: {}", file_path, commit_hash1, commit_hash2)));
                            return Err(JsError::from(err))
                        }
                    }
                    // 获取文件内容
                    let mut context1: String;
                    let context1_output = get_command_output("git", &repo, &["cat-file", "-p", &format!("{}:{}", commit_hash1, file_path)]);
                    match context1_output {
                        Ok(context1_output) => {
                            let stdout = String::from_utf8_lossy(&context1_output.stdout);
                            context1 = stdout.to_string();
                        }
                        Err(e) => {
                            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_path, commit_hash2)));
                            return Err(JsError::from(err))
                        }
                    };
                    let context2_output = get_command_output("git", &repo, &["cat-file", "-p", &format!("{}:{}", commit_hash2, file_path)]);
                    match context2_output {
                        Ok(context2_output) => {
                            let stdout = String::from_utf8_lossy(&context2_output.stdout);
                            context2 = stdout.to_string();
                        }
                        Err(e) => {
                            let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("")));
                            return Err(JsError::from(err))
                        }
                    };
                    Ok(FileDiffContext {
                        commit_hash1: commit_hash1.to_string(),
                        commit_hash2: commit_hash2.to_string(),
                        file_path: file_path.to_string(),
                        change_stat: FileLineChangeStat {
                            addition,
                            deletion,
                        },
                        context1,
                        context2,
                        file_status: status,
                    })
                }
                _ => {
                    Ok(FileDiffContext {
                        commit_hash1: commit_hash1.to_string(),
                        commit_hash2: commit_hash2.to_string(),
                        file_path: file_path.to_string(),
                        change_stat: FileLineChangeStat {
                            addition: 0,
                            deletion: 0,
                        },
                        context1: String::from(""),
                        context2: String::from(""),
                        file_status: status,
                    })
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            let err = napiError::from(io::Error::new(io::ErrorKind::Other, e));
            Err(JsError::from(err))
        }
    }
}

#[napi]
/**
 * get file content in a commit
 * @param repo repo path
 * @param commit_hash commit hash
 * @param file_path file path
 */
fn get_file_content (repo: String, commit_hash: String, file_path: String) -> Result<String, JsError> {
    let output = get_command_output("git", &repo, &["cat-file", "-p", &format!("{}:{}", commit_hash, file_path)]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.to_string())
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }
}

fn is_binary(content: &str) -> bool {
    // 判断前8000个字节中是否包含0
    for c in content.bytes().take(8000) {
        if c == 0 {
            return true;
        }
    }
    return false;
}

#[napi]
/**
 * get difference file diff statistic between two commits
 * @param repo repo path
 * @param commit_hash1 commit hash1
 * @param commit_hash2 commit hash2
 * @param file_path1 file path in commit1
 * @param file_path2 file path in commit2
 * @returns FileDiffContext
 */
fn get_diff_file_stat_between_commit(repo: String, commit_hash1: String, commit_hash2: String, file_path1: String, file_path2: String)-> Result<FileLineChangeStat, JsError> {
    let commit_range = format!("{}...{}", commit_hash1, commit_hash2);
    let output = get_command_output("git", &repo, &["diff", &commit_range, "--shortstat",  "--", &file_path1, &file_path2]);
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match log_shortstat_parse(&stdout) {
                Ok((_, insertation, deletion)) => {
                    Ok(FileLineChangeStat {
                        addition: insertation,
                        deletion: deletion,
                    })
                }
                Err(e) => {
                    let err = napiError::from(io::Error::new(io::ErrorKind::Other, "File to parse git diff shortstat"));
                    Err(JsError::from(err))
                }
            }
        }
        Err(e) => {
            let err = napiError::from(e);
            Err(JsError::from(err))
        }
    }


}

#[napi]
/**
 * get difference files diff between two commits
 * @param repo repo path
 * @param commit_hash1 commit hash1
 * @param commit_hash2 commit hash2
 * @returns FileDiffContext
 */
fn get_files_diff_context (repo: String, commit_hash1: String, commit_hash2: String) -> Result<Vec<FileDiffContext>, JsError> {
    let mut result = Vec::new();
    let files_status = get_files_status_between_commit(repo.to_string(), commit_hash1.to_string(), commit_hash2.to_string());
    match files_status {
        Ok(files_status) => {
            for file_status in files_status.iter() {
                // println!("{} {}", file_status.path, file_status.status);
                let mut file_content1 = String::from("");
                let mut file_content2 = String::from("");
                let mut addition = 0;
                let mut deletion = 0;
                match file_status.status {
                    FileStatusType::Added => {
                        let content = get_file_content(repo.to_string(), commit_hash2.to_string(), file_status.path.to_string());
                        match content {
                            Ok(content) => {
                                if is_binary(&content) {
                                    file_content1 = String::from("Binary file");
                                    file_content2 = String::from("Binary file");
                                } else {
                                    file_content2 = content;
                                    addition = file_content2.lines().count() as i32;
                                }
                            }
                            Err(_) => {
                                let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_status.path, commit_hash2)));
                                return Err(JsError::from(err))
                            }
                        }
                    }
                    FileStatusType::Deleted => {
                        let content = get_file_content(repo.to_string(), commit_hash1.to_string(), file_status.path.to_string());
                        match content {
                            Ok(content) => {
                                if is_binary(&content) {
                                    file_content1 = String::from("Binary file");
                                } else {
                                    file_content1 = content;
                                    deletion = file_content1.lines().count() as i32;
                                }
                            }
                            Err(_) => {
                                let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_status.path, commit_hash2)));
                                return Err(JsError::from(err))
                            }
                        }
                        file_content2 = String::from("File deleted");
                    }
                    FileStatusType::Modified => {
                        let content1 = get_file_content(repo.to_string(), commit_hash1.to_string(), file_status.path.to_string());
                        let content2 = get_file_content(repo.to_string(), commit_hash2.to_string(), file_status.path.to_string());
                        let file_change_stat = get_file_change_stat_between_commit(repo.to_string(), commit_hash1.to_string(), commit_hash2.to_string(), file_status.path.to_string());
                        match (content1, content2) {
                            (Ok(content1), Ok(content2)) => {
                                if is_binary(&content1) && is_binary(&content2) {
                                    file_content1 = String::from("Binary file");
                                    file_content2 = String::from("Binary file");
                                } else if is_binary(&content1) {
                                    file_content1 = String::from("Binary file");
                                    file_content2 = content2;
                                }else if is_binary(&content2) {
                                    file_content1 = content1;
                                    file_content2 = String::from("Binary file");
                                } else {
                                    file_content1 = content1;
                                    file_content2 = content2;
                                }
                            },
                            (_, _) => {
                                let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_status.path, commit_hash2)));
                                return Err(JsError::from(err))
                            }
                        }
                        match file_change_stat {
                            Ok(file_change_stat) => {
                                addition = file_change_stat.addition;
                                deletion = file_change_stat.deletion;
                            },
                            Err(e) => {
                                return Err(e)
                            }
                        }
                    }
                    FileStatusType::Renamed => {
                        let reg = Regex::new(r"\s*=>\s*").unwrap();
                        let names = reg.split(&file_status.message).collect::<Vec<&str>>();
                        let name1 = names[0];
                        let name2 = names[1];
                        let content1 = get_file_content(repo.to_string(), commit_hash1.to_string(), name1.to_string());
                        let content2 = get_file_content(repo.to_string(), commit_hash2.to_string(), name2.to_string());
                        let file_change_stat = get_diff_file_stat_between_commit(repo.to_string(), commit_hash1.to_string(), commit_hash2.to_string(), name1.to_string(), name2.to_string());
                        match (content1, content2) {
                            (Ok(content1), Ok(content2)) => {
                                if is_binary(&content1) && is_binary(&content2) {
                                    file_content1 = String::from("Binary file");
                                    file_content2 = String::from("Binary file");
                                } else if is_binary(&content1) {
                                    file_content1 = String::from("Binary file");
                                    file_content2 = content2;
                                }else if is_binary(&content2) {
                                    file_content1 = content1;
                                    file_content2 = String::from("Binary file");
                                } else {
                                    file_content1 = content1;
                                    file_content2 = content2;
                                }
                            }
                            (_, _) => {
                                let err = napiError::from(io::Error::new(io::ErrorKind::Other, format!("Failed to get file content:\nfile path: {}\ncommit hash: {}", file_status.path, commit_hash2)));
                                return Err(JsError::from(err))
                            }
                        }
                        match file_change_stat {
                            Ok(file_change_stat) => {
                                addition = file_change_stat.addition;
                                deletion = file_change_stat.deletion;
                            },
                            Err(e) => {
                                return Err(e)
                            }
                        }
                    }
                    _ => {}
                };
                result.push(FileDiffContext {
                    commit_hash1: commit_hash1.to_string(),
                    commit_hash2: commit_hash2.to_string(),
                    file_path: file_status.path.to_string(),
                    change_stat: FileLineChangeStat {
                        addition: addition,
                        deletion: deletion
                    },
                    context1: file_content1,
                    context2: file_content2,
                    file_status: file_status.status
                })
            }
            Ok(result)
        }
        Err(e) => {
            return Err(e)
        }
    }
}


#[cfg(test)]
mod tests {
    use core::time;

    use util::get_current_time;

    use super::*;

    #[test]
    fn test_get_commit_file_status() {    
        let path = String::from(r"E:\workSpace\Rust\rust_test");
        let hash = String::from("6a8a10948c80df189f79ff680df66d688b93bdd2");
        let res = get_commit_file_status(path.to_string(), hash.to_string());
        match res {
            Ok(res) => {
                println!("{:#?}", res);
            },
            Err(e) => {
            }
        }
    }
    #[test]
    fn test_get_file_commit_status() {
        let path = String::from(r"E:\workSpace\Rust\git-util-native");
        let commit_hash1 = String::from("2ffc252bee9edcdfa27d0689e4c9f4f80f72b608^");
        let commit_hash2 = String::from("2ffc252bee9edcdfa27d0689e4c9f4f80f72b608");
        let file_path = String::from("src/structs.rs");
        let res = get_file_between_commit_status(path.to_string(), commit_hash1.to_string(), file_path.to_string());
        match res {
            Ok(res) => {
                println!("{:#?}", res);
            },
            Err(e) => {
            }
        }
    }
    #[test]
    fn test_diff_file_context() {
        let path = String::from(r"E:\workSpace\JavaScript\giter");
        let commit_hash1 = String::from("fe2eff4^");
        let commit_hash2 = String::from("fe2eff4");
        let file_path = String::from("src/electron/workThreads/WorkPool.ts");
        let res = diff_file_context(path.to_string(), commit_hash1.to_string(), commit_hash2.to_string(), file_path.to_string());
        match res {
            Ok(res) => {
                println!("===============\n{:#?}\n=======================", res);
            },
            Err(e) => {
                println!("ERROR");
            }
        }
    }
    #[test]
    fn test_get_file_content() {
        let path = String::from(r"E:\workSpace\JavaScript\giter");
        let commit_hash = String::from("fe2eff4");
        let file_path = String::from("src/renderer/views/setting/index.vue");
        let res = get_file_content(path.to_string(), commit_hash.to_string(), file_path.to_string());
        match res {
            Ok(res) => {
                println!("===============\n{:#?}\n=======================", res);
            },
            Err(e) => {
                println!("ERROR");
            }
        }
    }

    #[test]
    fn test_get_files_diff_context() {
        let path = String::from(r"D:\work_project\JavaScript\giter");
        let commit1_hash = String::from("fe2eff4^");
        let commit2_hash = String::from("fe2eff4");
        let t1 = get_current_time();
        let res = get_files_diff_context(path.to_string(), commit1_hash.to_string(), commit2_hash.to_string());
        match res {
            Ok(res) => {
                let t2 = get_current_time();
                println!("time: {}", t2 - t1);
                // println!("===============\n{:#?}\n== =====================", res);
                for file_diff in res {
                    println!("====================================");
                    println!("{}", file_diff.file_path);
                    println!("{}", file_diff.file_status);
                    println!("{}", file_diff.change_stat);
                }
            },
            Err(e) => {
                println!("ERROR");
            }
        }
    }
}