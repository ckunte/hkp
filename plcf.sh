#!/usr/bin/env bash
echo -n "Purging LaTeX compile files..."
find ~/. -name "*.log" -type f -delete
find ~/. -name "*.out" -type f -delete
find ~/. -name "*.aux" -type f -delete
find ~/. -name "*.gz" -type f -delete
echo "done."