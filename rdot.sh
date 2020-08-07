#!/usr/bin/env bash
echo -n "Removing periods (.) from folders and pdf files..."
# Remove periods (.) from folder names
fd --type d . | rename 's/\.(?=[^.]*)/_/g'
# Remove periods (.) from pdf files
fd --type f -e pdf . | rename 's/\.(?=[^.]*\.)/_/g'
# Remove periods (.) from pdf files
fd --type f -e docx . | rename 's/\.(?=[^.]*\.)/_/g'
echo "done."