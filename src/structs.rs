use napi_derive::napi;
use core::hash::Hash;
use std::{cmp::Eq, collections::{hash_map::RandomState, HashMap}, string};


#[napi(object)]
#[derive(Clone, Eq)]
pub struct Author {
    pub name: String,
    pub email: String
}

impl PartialEq for Author {
    fn eq(&self, other: &Author) -> bool {
        self.name == other.name && self.email == other.email
    }
}

impl Hash for Author {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.email.hash(state);
    }
}



#[napi(object)]
#[derive(Clone)]
pub struct BranchCreatedInfo {
    pub name: String,
    pub time: String,
    pub author: Author,
    pub hash: String
}

#[napi(object)]
#[derive(Clone)]
pub struct Branch {
    pub name: String,
    pub created: BranchCreatedInfo,
    pub authors: Vec<Author>,
}

#[napi(object)]
#[derive(Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub operate: Vec<String>
}

#[napi(object)]
#[derive(Clone)]
pub struct RepositoryFull {
    pub current_branch: Branch,
    pub branches: Vec<Branch>,
    pub authors: Vec<Author>,
    pub name: String,
    pub remote: Vec<Remote>,
    pub path: String 
}

#[napi(object)]
#[derive(Clone)]
pub struct RepositorySimple {
    pub name: String,
    pub path: String,
    pub branches: Vec<String>,
    pub current_branch: String,
    pub remote: Vec<Remote>,
    pub authors: Vec<Author>,

}

#[napi(object)]
#[derive(Clone)]
/**
 * The statistic of daily contribute in a branch
 */
pub struct StatDailyContribute {
    pub commit_count: i32,
    pub data_list: Vec<String>,
    pub insertion: Vec<i32>,
    pub deletions: Vec<i32>,
    pub change_files: Vec<i32>
}

#[napi(object)]
#[derive(Clone)]
pub struct AuthorStatDailyContribute {
    pub author : Author,
    pub stat: StatDailyContribute,
}

#[napi(object)]
#[derive(Clone)]
pub struct BranchStatDailyContribute {
    pub branch: String,
    pub total_stat: StatDailyContribute,
    pub authors_stat: Vec<AuthorStatDailyContribute>,
}

#[napi(object)]
#[derive(Clone)]
pub struct RepoFileInfo {
    pub name: String,
    pub dir: String,
    pub object_mode: String,
    pub object_type: String,
    pub object_name: String,
    pub object_size: String,
    pub is_dir: bool,
    pub children: Vec<RepoFileInfo>
}