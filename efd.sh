#!/usr/bin/env bash
echo -n "Removing empty folders..."
find . -type d -empty -execdir rmdir {} +
echo "done."