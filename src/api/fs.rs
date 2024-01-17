use std::error::Error;
use std::fs;
use std::path::PathBuf;

use git2::Repository;

pub struct DotfileStorage {
    repo_path: PathBuf,
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
            repo_path: path.to_owned(),
            repo,
            tracked_files,
        })
    }
    pub fn track_file(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error +Send +Sync>> {
        if !path.as_path().is_file() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "File does not exist")));
        }
        let filename = path.as_path().file_name().ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "File does not exist"))?;
        let mut repo_location = self.repo_path.to_owned();
        repo_location.push(filename);
        fs::copy(path, repo_location).map(|_r| self.tracked_files.push(filename.to_string_lossy().to_string())).map_err(|e| Box::new(e) as Box<dyn Error +Send +Sync>)
    }
}
