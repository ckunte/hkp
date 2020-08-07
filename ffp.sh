#!/usr/bin/env bash
echo -n "Applying permissions: folders (755), files (644)..."
find . -type d -exec chmod 755 {} +
find . -type f -exec chmod 644 {} +
echo "done."