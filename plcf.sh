#!/usr/bin/env bash
echo -n "Purging LaTeX compile files..."
find . -type f -name "*.aux" -delete
find . -type f -name "*.gz" -delete
find . -type f -name "*.lof" -delete
find . -type f -name "*.log" -delete
find . -type f -name "*.lot" -delete
find . -type f -name "*.out" -delete
find . -type f -name "*.toc" -delete
echo "done."
