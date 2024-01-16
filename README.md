# cfgtool - A simple git wrapper to manage your dotfiles
## Functionality
### Tracking a file
Use ```cfgtool track /path/to/file``` to start tracking a file.
### Updating a file
Use ```cfgtool update /path/to/file``` to create a new commit with your config changes. You can also specify multiple files to be parts. Alternatively use ```cfgtool update -a``` to update all files.
### Syncing with a remote
Use ```cfgtool sync``` to sync your local commits with a remote git repo.
### Rollback
Use ```cfgtool rollback``` to rollback a config file to its previous state.
