cargo build
alias cfgtool="$(pwd)/target/debug/cfgtool"
rm -rf ~/.local/share/cfgtool
rm -rf /tmp/cfgtool-repo
mkdir /tmp/cfgtool-repo
git init --bare /tmp/cfgtool-repo
echo "test file" > cfgfile.example
cfgtool track cfgfile.example
gitdir=~/.local/share/cfgtool/repo
git --git-dir=$gitdir/.git --work-tree=$gitdir remote add origin /tmp/cfgfile-repo
