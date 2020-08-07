#!/usr/bin/env zsh
# 2014 ckunte
# Convert Markdown files in a folder to plain text
# Requires pandoc and zsh.
#
# Run the following at zsh prompt:
#
#  $ zsh md2txt.sh
#
for file in *.md; do pandoc -f markdown -t plain --wrap=none "$file" -o "$file.txt"; done
autoload zmv
zmv '(*).md.txt' '$1.txt'
