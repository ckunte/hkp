#!/usr/bin/env bash

# Function to combine PDF files in a folder
combine_pdfs() {
  local folder="$1"
  local combined_file="${folder##*/}-comb.pdf"  # Use folder name as prefix
  
  # Move to the folder
  cd "$folder" || exit
  
  # Combine PDF files
  gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -sOutputFile="$combined_file" *.pdf
  
  # Return to the original directory
  cd - || exit
}

# Function to process folders
process_folders() {
  local root_folder="$1"
  
  # Find all subfolders
  find "$root_folder" -type d | while read -r folder; do
    # Combine PDFs in each folder
    combine_pdfs "$folder"
  done
}

# Main script
if [ "$#" -eq 1 ]; then
  root_folder="$1"
  if [ -d "$root_folder" ]; then
    process_folders "$root_folder"
    echo "PDF files combined successfully in all folders."
  else
    echo "Error: The specified path is not a directory."
    exit 1
  fi
else
  echo "Usage: $0 <root_folder>"
  exit 1
fi
