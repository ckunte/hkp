#!/usr/bin/env bash

# Find all empty directories in the current folder and its subfolders
empty_dirs=$(find . -type d -empty)

# Prompt user for confirmation
read -p "Remove all empty directories in $(pwd) and its subfolders? [y/n] " choice
case "$choice" in 
  y|Y ) # User chose to remove directories
        # Remove each empty directory found
        for dir in $empty_dirs; do
          rmdir "$dir"
          echo "Removed empty directory: $dir"
        done
        ;;
  n|N ) # User chose not to remove directories
        echo "No directories removed."
        ;;
  * )   # User did not enter a valid choice
        echo "Invalid choice. No directories removed."
        ;;
esac
