use api::fs::DotfileStorage;
use clap::{arg, value_parser, Arg, ArgMatches, Command};
use std::{
    error::Error,
    io::{self, BufRead, Write}, path::PathBuf,
};
mod api;

fn main() {
    let cmd = Command::new("cfgtool")
        .bin_name("cfgtool")
        .version("0.1.0")
        .about("A simple git wrapper to manage your dotfiles")
        .subcommand_required(true)
        .subcommand(
            Command::new("track")
                .about("Track a file with cfgtool.")
                .arg(
                    arg!(<PATH>)
                        .id("path")
                        .help("The path of the file to track.")
                        .required(true)
                        .value_parser(value_parser!(std::path::PathBuf)),
                ),
        )
        .subcommand(Command::new("update").about("Update all tracked files."))
        .subcommand(Command::new("sync").about("Sync your dotfiles with a remote")
                    .arg(Arg::new("force")
                         .short('f')
                         .long("force")
                         .num_args(0)
                         .help("Force the sync omitting local changes.")
                         .required(false)))
        .subcommand(Command::new("rollback").about("Rollback a file to a previous version"))
        .subcommand(Command::new("status").about("Display status information about tracked files."));
    let matches = cmd.get_matches();
    handle_command(&matches).expect("Error handling command.");
}

fn handle_command(matches: &ArgMatches) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut dotfile_repo_path =
        dirs::data_dir().expect("Critical Failure while trying to create config dir");

    // Initialise the dotfile repo struct
    dotfile_repo_path.push("cfgtool");
    dotfile_repo_path.push("repo");
    let mut dotfile_repo = DotfileStorage::new(&dotfile_repo_path).unwrap();
    // Get stdin
    let stdin = io::stdin();

    let response = match matches.subcommand().expect("This should never happen.") {
        ("track", subcommand_match) => {
            if dotfile_repo.is_tracked(
                subcommand_match
                    .get_one("path")
                    .expect("Should never happen."),
            ) {
                println!("File is already tracked.");
                Ok(())
            } else {
                dotfile_repo.track_file(
                    subcommand_match
                        .get_one("path")
                        .expect("Should never happen."),
                    None,
                )
            }
        }
        ("update", _subcommand_match) => {
            if dotfile_repo.get_changed_files(true)?.len() == 0 {
                println!("Nothing has changed.");
            }
            for file in dotfile_repo.get_changed_files(true)? {
                print!(
                    "File {} has changed. Do you want to track the changes? (y/n) ",
                    file.to_owned()
                        .into_os_string()
                        .to_string_lossy()
                        .to_string()
                );
                //  Ensure text gets printed
                io::stdout().flush()?;
                // Cant directly match the response because then stdin won't get unlocked
                let response = stdin.lock().lines().next().unwrap().unwrap();
                match response.as_str() {
                    "y" => {
                        println!("Please describe your changes:");
                        let msg = stdin.lock().lines().next().unwrap().unwrap();
                        dotfile_repo.track_file(&file, Some(&msg))?;
                    }
                    "n" => println!("Ignoring..."),
                    _ => println!("Invalid input given. Skipping for now."),
                }
            }
            Ok(())
        }
        ("sync", subcommand_match) => {
            if dotfile_repo.get_changed_files(true)?.len() != 0 && !subcommand_match.get_one::<bool>("force").unwrap() {
                println!("Warning you still have untracked local changes. Syncing now would overwrite them. Run the force flag to ignore.")
            } else {
                match dotfile_repo.get_default_remote() {
                    Some(remote) => println!("Default remote {remote}."),
                    None => {
                        print!("No remote to sync known. Do you want to add one? (y/n) ");
                        //  Ensure text gets printed
                        io::stdout().flush()?;
                        // Cant directly match the response because then stdin won't get unlocked
                        let response = stdin.lock().lines().next().unwrap().unwrap();
                        match response.as_str() {
                            "y" => {
                                println!("Please enter the remotes url:");
                                let url = stdin.lock().lines().next().unwrap().unwrap();
                                dotfile_repo.add_remote("origin", &url)?;
                            }
                            "n" => println!("Ignoring..."),
                            _ => println!("Invalid input given. Skipping for now."),
                        }
                    }
                }
                // TODO: Respect diff between local files and local repo.
                // Currently these diffs will get overwritten on sync
                match dotfile_repo.pull_main() {
                    Ok(_) => dotfile_repo.push_main()?,
                    // Continue to push if pull failed because the remote  is empty
                    Err(e) => {
                        if e.to_string() == "corrupted loose reference file: FETCH_HEAD" {
                            dotfile_repo.push_main()?
                        }
                    }
                }
                dotfile_repo.copy_repo_to_local()?;
            }
            Ok(())
        }
        ("status", _subcommand_match) => {
            let tracked = dotfile_repo.get_tracked_files()?;
            if tracked.len() == 0 {
                println!("Nothing tracked yet...");
            } else {
                for file in tracked {
                    println!("{} at location: {}", file.file_name().unwrap().to_string_lossy(), file.as_path().iter().skip(3).collect::<PathBuf>().display());
                }
            }
            Ok(())
        },
        ("rollback", _subcommand_match) => {
            println!("Not yet implemented.");
            Ok(())
        }
        (_, _) => Ok(()),
    };
    if response.is_err() {
        let err = response.err().unwrap();
        println!("Error: {}", err.to_string());
    }
    Ok(())
}
