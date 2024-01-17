use api::fs::DotfileStorage;
use clap::{Command, arg, value_parser, ArgMatches};
use std::error::Error;
mod api;

fn main() {
    let cmd = Command::new("cfgtool")
        .bin_name("cfgtool")
        .version("0.0.1")
        .about("A simple git wrapper to manage your dotfiles")
        .subcommand_required(true)
        .subcommand(Command::new("track")
                    .about("Track a file with cfgtool.")
                    .arg(arg!(<PATH>)
                         .id("path")
                         .help("The path of the file to track.")
                         .required(true)
                         .value_parser(value_parser!(std::path::PathBuf))
                         )
                    )
        .subcommand(Command::new("update")
                    .about("Update a tracked file."))
        .subcommand(Command::new("sync")
                    .about("Sync your dotfiles with a remote"))
        .subcommand(Command::new("rollback")
                    .about("Rollback a file to a previous version"));
    let matches = cmd.get_matches();
    handle_command(&matches).expect("Error handling command.");
}

fn handle_command(matches: &ArgMatches) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut dotfile_repo_path = dirs::data_dir().expect("Critical Failure while trying to create config dir");

    // Initialise the dotfile repo struct
    dotfile_repo_path.push("cfgtool");
    dotfile_repo_path.push("repo");
    let mut dotfile_repo = DotfileStorage::new(&dotfile_repo_path).unwrap();

    match matches.subcommand().expect("This should never happen.") {
        ("track", subcommand_match) => dotfile_repo.track_file(subcommand_match.get_one("path").expect("Should never happen.")),
        (_, _) => Ok(())
        
    }
}
