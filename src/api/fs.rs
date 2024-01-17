use std::error::Error;
use std::fs;
use std::path::PathBuf;

use git2::Repository;

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
        let index = repo.index()?;
        let tracked_files = index
            .iter()
            .map(|e| String::from_utf8(e.path))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .collect();
        Ok(DotfileStorage {
            path: path.to_owned(),
            repo,
            tracked_files,
        })
    }
}
