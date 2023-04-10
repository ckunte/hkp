#!/usr/bin/env bash
# Get the current folder name
folder=$(basename "$(pwd)")
echo -n "Combining all pdf files in the curr folder..."
gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -sOutputFile="$folder-comb.pdf" *.pdf
echo "done."
echo "Combined file: $folder-comb.pdf"
