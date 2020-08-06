extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
extern crate git2;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempfile;
extern crate thiserror;
extern crate urlencoding;
extern crate uuid;

use dotenv::dotenv;
use env_logger::Env;

use std::fs::File;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::{env, fs, io, thread, time};

use chrono::TimeZone;
use git2::ConfigLevel::App;
use git2::ErrorClass::Reference;
use git2::{
    BranchType, Commit, DiffFormat, DiffOptions, Direction, IndexEntry, IndexTime, ObjectType, Oid,
    Repository, Signature, Time, TreeWalkMode, TreeWalkResult,
};
use path_clean::{clean, PathClean};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::export::Result::Err;
use tempfile::TempDir;
use uuid::Uuid;

use crate::errors::AppError;

mod errors;

pub enum AbsolutePath<'a> {
    Repository(&'a str, Option<String>),
    Workspace(Option<&'a str>, Option<String>),
}

pub fn random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .collect()
}

pub fn uuid() -> String {
    Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_string()
}

fn storage_path() -> String {
    "./storage".to_string()
}

fn absolute_path(path_type: &AbsolutePath) -> Result<String, AppError> {
    let current_dir =
        env::current_dir().expect("Failed to get env current dir while it always should!");

    let storage_path = storage_path();

    let id_ = uuid();
    let (id, sub_dir) = match &path_type {
        AbsolutePath::Repository(repo_id_, _) => {
            let id = repo_id_.to_owned();
            (id, "repositories")
        }
        AbsolutePath::Workspace(_, _) => (id_.as_str(), "workspace"),
    };

    let dir = &id[0..2];
    let path = current_dir
        .join(&storage_path)
        .join(sub_dir)
        .join(dir)
        .join(&id);

    let final_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir().unwrap().join(path)
    }
    .clean();

    final_path
        .into_os_string()
        .into_string()
        .map_err(|e| AppError::CommandError(format!("{:?}", e)))
}

pub fn repo_create(repo_id: &str) -> Result<Repository, AppError> {
    let repo_path = repository_path(repo_id)?;
    Repository::init_bare(&repo_path).map_err(|e| AppError::Git2Error(e))
}

pub fn repository_path(repo_id: &str) -> Result<String, AppError> {
    absolute_path(&AbsolutePath::Repository(repo_id, None))
}

pub fn open_bare_repo(repo_id: &str) -> Result<Repository, AppError> {
    let repo_path = repository_path(repo_id)?;
    Repository::open_bare(&repo_path).map_err(|e| AppError::Git2Error(e))
}

pub fn last_commit_id_of_repo(repo: &Repository) -> Result<Option<Oid>, AppError> {
    Ok(repo.head()?.target())
}

pub fn last_commit_id_of_branch(repo: &Repository, branch_name: &str) -> Result<String, AppError> {
    let branch = repo.find_branch(&branch_name, BranchType::Local)?;
    let reference = branch.into_reference();
    let target = reference.target();
    match target {
        Some(oid) => Ok(oid.to_string()),
        None => Err(AppError::CommandError(format!(
            "Failed to get target of branch {}",
            branch_name
        ))),
    }
}

pub fn commit(
    repo: &Repository,
    branch_name: Option<&str>,
    index_tree_id: &Oid,
    author_signature: &Signature,
    committer_signature: &Signature,
    message: &str,
) -> Result<Oid, AppError> {
    let repo_path = repo.path().to_str().unwrap();
    let repo = Repository::open_bare(repo_path)?;

    let mut parents = vec![];
    let parent_commit;
    if let Ok(_head) = repo.head() {
        let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
        parent_commit = obj.into_commit().expect("Failed to get parent commit!");
        parents.push(&parent_commit);
    }

    let reference = if repo.is_empty()? {
        "HEAD".to_string()
    } else {
        match branch_name {
            Some(bn) => {
                let branch = repo.find_branch(&bn, BranchType::Local)?;
                let b_ref = branch.into_reference();
                match b_ref.name() {
                    Some(bn) => bn.clone().to_string(),
                    None => {
                        return Err(AppError::CommandError(format!(
                            "Failed to get reference name for branch {}",
                            &bn
                        )));
                    }
                }
            }
            None => "HEAD".to_string(),
        }
    };

    let index_tree = repo.find_tree(index_tree_id.clone())?;

    let oid = match repo.commit(
        Some(&reference.as_str()), // point HEAD to our new commit
        &author_signature,         // author
        &committer_signature,      // committer
        message,                   // commit message
        &index_tree,               // tree
        &parents[..],
    ) {
        Ok(oid) => oid,
        Err(e) => {
            thread::sleep(time::Duration::from_millis(1000));
            commit(
                &repo,
                branch_name,
                &index_tree_id,
                &author_signature,
                &committer_signature,
                &message,
            )?
        }
    };

    Ok(oid)
}

fn make_index_entry() -> IndexEntry {
    IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: Oid::from_bytes(&[0; 20]).unwrap(),
        flags: 0,
        flags_extended: 0,
        path: Vec::new(),
    }
}

pub fn create_file_then_commit_then_push(
    repo: &Repository,
    file_path: &str,
    file_content: &str,
    branch: &str,
    author_signature: &Signature,
    committer_signature: &Signature,
    commit_message: &str,
) -> Result<Oid, AppError> {
    let repo_path = repo.path().to_str().unwrap().to_string();

    // Wait if the repo is locked
    let mut i: usize = 0;
    while is_repo_locked(&repo_path) {
        if i >= 20000 {
            return Err(AppError::InternalServerError(format!(
                "Repository({}) is locked!",
                &repo_path
            )));
        }
        println!(">>>>>>>>>>>>>> waiting repo {}", &repo_path);
        thread::sleep(time::Duration::from_millis(1000));
        i = i + 1;
    }

    thread::sleep(time::Duration::from_millis(1000));

    // lock the repo by a creating a specified file
    lock_repo(&repo_path);

    let mut index = repo.index()?;

    let mut index_entry = make_index_entry();
    index_entry.path = file_path.as_bytes().to_vec();
    let content = file_content.as_bytes();
    index.add_frombuffer(&index_entry, content)?;
    let index_tree_id = index.write_tree()?;
    let commit_id = commit(
        &repo,
        Some(branch),
        &index_tree_id,
        &author_signature,
        &committer_signature,
        &commit_message,
    )?;

    unlock_repo(&repo_path);

    return Ok(commit_id);
}

pub fn list_files_of_branch(
    repo: &Repository,
    branch: &str,
    path: Option<&str>,
) -> Result<Vec<String>, AppError> {
    let b = if branch.is_empty() { "master" } else { branch };
    let commit_id = last_commit_id_of_branch(&repo, &b)?;

    let cmt_id;
    if commit_id.is_empty() {
        match last_commit_id_of_repo(&repo)? {
            Some(oid) => cmt_id = oid.to_string(),
            None => return Ok(vec![]),
        }
    } else {
        cmt_id = commit_id.to_string();
    }

    let commit_id = Oid::from_str(&cmt_id)?;

    let commit = match repo.find_commit(commit_id) {
        Ok(commit) => commit,
        Err(e) => {
            return Err(AppError::Git2Error(e));
        }
    };

    let commit_tree = commit.tree()?;

    let tree = match path {
        Some(p) => {
            let entry = commit_tree
                .get_path(Path::new(&p))
                .map_err(|e| AppError::BadRequestError(e.message().to_string()))?;
            repo.find_tree(entry.id())?
        }
        None => commit_tree,
    };

    let mut files = tree
        .iter()
        .filter_map(|entry| {
            let name_bytes = entry.name_bytes();
            let filename = String::from_utf8_lossy(name_bytes).to_string();
            Some(filename)
        })
        .collect::<Vec<String>>();

    Ok(files)
}

fn repo_lock_file_path(repo_path: &str) -> PathBuf {
    Path::new(&repo_path).with_extension("lock")
}

fn is_repo_locked(repo_path: &str) -> bool {
    repo_lock_file_path(&repo_path).exists()
}

fn lock_repo(repo_path: &str) -> Result<(), AppError> {
    let lock_file_path = repo_lock_file_path(&repo_path);
    File::create(&lock_file_path)?;
    Ok(())
}

fn unlock_repo(repo_path: &str) -> Result<(), AppError> {
    let lock_file_path = repo_lock_file_path(&repo_path);
    fs::remove_file(&lock_file_path)?;
    Ok(())
}

fn main() {}

#[cfg(test)]
mod tests {
    use std::thread;

    use serde::de::Expected;
    use serde_json::Value;

    use super::*;

    /// Try to create files from multiple threads, but all old files will be deleted for known reason
    #[test]
    fn test_create_file_and_commit() {
        let repo_id = uuid();

        println!(">>>>>>>>>>>>>>>>> repo_id: {}", &repo_id);

        let repo = repo_create(&repo_id).unwrap();

        let count = 10;

        let mut children = vec![];

        // From multiple threads
        for i in 0..count {
            // Spin up another thread
            let repo_id = repo_id.clone();
            children.push(thread::spawn(move || {
                let signature = Signature::now("Zhang", "zhang@gmail.com").unwrap();
                let branch = "master";

                let file_path = random_string(10);
                let file_content = random_string(100);
                let commit_message = random_string(100);

                let repo = open_bare_repo(&repo_id.clone()).unwrap();

                match create_file_then_commit_then_push(
                    &repo,
                    &file_path,
                    &file_content,
                    &branch,
                    &signature,
                    &signature,
                    &commit_message,
                ) {
                    Ok(a) => {}
                    Err(e) => println!(">>>>>>>>>>>> got error: {:?}", e),
                }
            }));
        }

        for child in children {
            // Wait for the thread to finish. Returns a result.
            let _ = child.join();
        }

        let files = list_files_of_branch(&repo, "master", None).unwrap();
        assert_eq!(files.len(), count);
    }
}
