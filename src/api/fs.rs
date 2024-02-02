use std::rc::Rc;
use std::error::Error;
use std::ffi::{CString, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use git2::string_array::StringArray;
use git2::{Index, Repository, Status, Commit};

pub struct DotfileStorage {
    repo_path: PathBuf,
    repo: Rc<Repository>,
    tracked_files: Vec<String>,
    remotes: Vec<String>
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
        let remotes = repo.remotes().map(|f| string_arry_to_vec(f))?;
        Ok(DotfileStorage {
            repo_path: path.to_owned(),
            repo: Rc::new(repo),
            tracked_files,
            remotes,
        })
    }
    pub fn track_file(&mut self, path: &PathBuf, commit_msg: Option<&str>) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !path.as_path().is_file() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File does not exist",
            )));
        }
        // Convert relative path to absolute path
        let path = &fs::canonicalize(path)?;
        DotfileStorage::validate_filepath(path)?;

        let filename = path.as_path().file_name().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "File does not exist",
        ))?;
        let repo_location: PathBuf = self
            .repo_path
            .to_owned()
            .iter()
            .chain(
                path.iter().skip(
                    dirs::home_dir()
                        .expect("Operating system not supportet")
                        .iter()
                        .count(),
                ),
            )
            .collect();
        fs::create_dir_all(
            repo_location
                .as_path()
                .parent()
                .expect("Criticical fs error."),
        )?;
        fs::copy(path, repo_location)
            .map(|_r| {
                self.tracked_files
                    .push(filename.to_string_lossy().to_string())
            })
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        let default_msg = format!("Tracked file {}", filename.to_string_lossy());
        let commit_msg = commit_msg.unwrap_or(&default_msg);
        self.add_and_commit(&path.into_iter().skip(3).collect::<PathBuf>().as_path(), &commit_msg)?;
        Ok(())
    }
    // Helper function that returns all files  currently tracked by the git repo
    fn get_tracked_files(&self) -> Result<Vec<PathBuf>, Box<dyn Error +Send +Sync>> {
        let paths = self.repo.index()?.iter()
            .map(|c| CString::new(c.path).unwrap().into_string().unwrap())
            .map(|c| {
            let mut path = self.repo_path.to_owned();
            for part in c.split("/") {
                path.push(part);
            }
            path
        }).collect();
        Ok(paths)
    }

    // Function that returns all tracked file that have a diff to the git repo
    pub fn get_changed_files(&self) -> Result<Vec<PathBuf>, Box<dyn Error +Send +Sync>> {
        Ok(self.get_tracked_files()?.into_iter()
            .filter(|f| {
                // The local counterpart to the tracked file
                let local_counterpart: PathBuf = dirs::home_dir().unwrap().as_path().iter()
                    .chain(f.to_owned().as_path().iter().skip(self.repo_path.as_path().iter().count())).collect();
                // Reads both files into a string
                let repo_file = fs::read_to_string(f).unwrap();
                let local_file = fs::read_to_string(local_counterpart).unwrap();
                // Checks for diff
                repo_file != local_file
            })
            // Convert every remaining path into their local counterpart
            .map(|f| dirs::home_dir().unwrap().as_path().iter().chain(f.as_path().iter().skip(self.repo_path.as_path().iter().count())).collect::<PathBuf>())
            .collect())
    }

    // Helper function that adds a file to the index and then commits.
    fn add_and_commit(&self, path: &Path, msg: &str) -> Result<(), Box<dyn Error +Send +Sync>> {
        // Adds the file to the index.
        let mut index = self.repo.index()?;
        index.add_path(path)?;
        index.write()?;

        // Creates tree from index (necessary to create a commit)
        let tree = self.repo.find_tree(index.write_tree()?)?;

        // Checks if the repo has an initial commit
        if self.repo.head().is_ok() {
            // Initial commit exists
            // Creates the actual commit
            self.repo.commit(Some("HEAD"), &self.repo.signature()?, &self.repo.signature()?, msg, &tree, &[&self.find_latest_commit()?])?;
        } else {
            // Initial Commit doesn't exist
            // Creates initial commit
            self.repo.commit(Some("HEAD"), &self.repo.signature()?, &self.repo.signature()?, msg, &tree, &vec![])?;
        }
        Ok(())
    }

    fn find_latest_commit(&self) -> Result<Commit, Box<dyn Error +Send +Sync>> {
        let obj = self.repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        obj.into_commit().map_err(|_| Box::new(git2::Error::from_str("Could't find commit.")) as Box<dyn Error +Send +Sync>)
    }

    // Checks if the given path is in the homedir
    fn validate_filepath(path: &PathBuf) -> std::io::Result<()> {
        match path.as_path().starts_with(
            dirs::home_dir()
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Config files outside the homedir aren't supported yet.",
                ))?
                .into_os_string(),
        ) {
            true => Ok(()),
            false => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Config files outside the homedir aren't supported yet.",
            )),
        }
    }
    pub fn is_tracked(&mut self, path: &PathBuf) -> bool {
        let path = fs::canonicalize(path).unwrap();
        let repo_location: PathBuf = self
            .repo_path
            .to_owned()
            .iter()
            .chain(
                path.iter().skip(
                    dirs::home_dir()
                        .expect("Operating system not supportet")
                        .iter()
                        .count(),
                ),
            )
            .collect();
        Path::new(&repo_location).exists()
    }
    pub fn get_default_remote(&self) -> Option<&str> {
        if self.remotes.iter().any(|f| f == &"origin") {
            Some("origin")
        } else if self.remotes.len() != 0 {
            self.remotes.get(0).map(|f| f.as_str())
        } else  {
            None
        }
    }
}

fn string_arry_to_vec(arr: StringArray) -> Vec<String> {
    arr.into_iter().map(|f| f.map(|g| g.to_string()).unwrap()).collect()
}
