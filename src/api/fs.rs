use std::rc::Rc;
use std::error::Error;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use git2::build::CheckoutBuilder;
use git2::string_array::StringArray;
use git2::{Repository, Commit};

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
    pub fn get_tracked_files(&self) -> Result<Vec<PathBuf>, Box<dyn Error +Send +Sync>> {
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
    pub fn get_changed_files(&self, localise_paths: bool) -> Result<Vec<PathBuf>, Box<dyn Error +Send +Sync>> {
        let changed = self.get_tracked_files()?.into_iter()
            .filter(|f| {
                // The local counterpart to the tracked file
                let local_counterpart: PathBuf = dirs::home_dir().unwrap().as_path().iter()
                    .chain(f.to_owned().as_path().iter().skip(self.repo_path.as_path().iter().count())).collect();
                // Reads both files into a string
                let repo_file = fs::read_to_string(f).unwrap();
                let local_file = fs::read_to_string(local_counterpart).unwrap();
                // Checks for diff
                repo_file != local_file
            });
        Ok(if localise_paths {
            // Convert every remaining path into their local counterpart
            changed.map(|f| dirs::home_dir().unwrap().as_path().iter().chain(f.as_path().iter().skip(self.repo_path.as_path().iter().count())).collect::<PathBuf>())
                .collect()
        } else {
            changed.collect()
        }) 
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
    pub fn add_remote(&mut self, name: &str, url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.repo.remote_set_url(name, url)?;
        if !self.remotes.iter().any(|f| f == name) {
            self.remotes.push(name.to_string());
        }
        Ok(())

    }
    // Fetches and fast-forwards the main branch of the default remote
    pub fn pull_main(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut remote = self.repo.find_remote(self.get_default_remote().ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No remote found.")))?)?;
        remote.fetch(&["main"], None, None)?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;
        let analysis = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            Ok(())
        } else if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{}", "main");
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-Forward")?;
            self.repo.set_head(&refname)?;
            self.repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Only fast-forward supported.")))
        }
    }

    pub fn push_main(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut remote = self.repo.find_remote(self.get_default_remote().ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No remote found.")))?)?;
        let branch = self.repo.branches(None)?
            .filter(|f| f.as_ref().unwrap().0.name().unwrap().unwrap() == "main")
            .last()
            .ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No main branch found.")))??.0;
        let branch_ref = branch.into_reference();
        let branch_ref_name = branch_ref.name().unwrap();
        remote.push(&[branch_ref_name], None)?;
        Ok(())
    }

    // Copy every file with diffs from the repo to its local counterpart.
    // This will overwrite unstaged changes
    pub fn copy_repo_to_local(&mut self) -> Result<(), Box<dyn Error +Send +Sync>> {
        for file in self.get_changed_files(false)? {
            let local_file = dirs::home_dir().unwrap().as_path().iter().chain(file.as_path().iter().skip(self.repo_path.as_path().iter().count())).collect::<PathBuf>();
            fs::copy(file, local_file)?;
        }
        Ok(())
    }
}

fn string_arry_to_vec(arr: StringArray) -> Vec<String> {
    arr.into_iter().map(|f| f.map(|g| g.to_string()).unwrap()).collect()
}
