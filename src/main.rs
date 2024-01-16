use api::fs::DotfileStorage;
use clap::{Command, command, arg, value_parser, Arg, Subcommand, ArgMatches};
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
    handle_command(matches.subcommand().expect("Clap should ensure that this never happens.").1);
}

fn handle_command(matches: &ArgMatches) {
    let mut dotfile_repo_path = dirs::data_dir().expect("Critical Failure while trying to create config dir");
    dotfile_repo_path.push("cfgtool");
    dotfile_repo_path.push("repo");

    let dotfile_repo = DotfileStorage::new(&dotfile_repo_path).unwrap();
}
