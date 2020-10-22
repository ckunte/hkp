#!/usr/bin/env bash
echo -n "Deleting .DS_Store files..."
find . -type f -name '.DS_Store' -exec rm {} \;
echo "done."