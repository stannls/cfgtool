use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

use git2::Repository;
use git2::TreeWalkResult;

pub struct DotfileStorage {
    path: PathBuf,
    repo: Repository,
    tracked_files: Vec<String>,
}

impl DotfileStorage {
    pub fn new(path: &PathBuf) -> Result<DotfileStorage, Box<dyn Error + Send + Sync>> {
        fs::create_dir_all(path)?;
        // Try to either open or init a git repo and throw an error if not possible.
        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(_) => match Repository::init(path) {
                Ok(repo) => repo,
                Err(e) => return Err(Box::new(e)),
            },
        };
        let mut tracked_files = vec![];
        // Introduce new scope because of borrow checker shenanigans.
        {
            let main_rev = repo.revparse_single("HEAD")?;
            let tree = main_rev.as_tree().ok_or(std::io::Error::new(
                io::ErrorKind::Other,
                "Error creating repository tree.",
            ))?;
            let _ = tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
                match entry.name() {
                    Some(name) => tracked_files.push(name.to_string()),
                    _ => {}
                }
                TreeWalkResult::Ok
            });
        };
        Ok(DotfileStorage {
            path: path.to_owned(),
            repo,
            tracked_files,
        })
    }
}
