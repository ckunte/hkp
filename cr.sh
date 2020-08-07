#!/usr/bin/env zsh
git checkout --orphan newBranch
git add -A
git commit -m 'first commit'
# Deletes the master branch
git branch -D master
# Rename the current branch to master
git branch -m master
# Force push master branch to github
git push -f origin master
# Remove old files
git gc --aggressive --prune=all