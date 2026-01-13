#!/bin/bash

# Folder containing PNGs (default: current folder)
INPUT_DIR="${1:-.}"

# Output folder for compressed files
OUTPUT_DIR="${2:-compressed}"

# Create output folder if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Loop through all PNG files
for file in "$INPUT_DIR"/*.png; do
    [ -e "$file" ] || continue  # Skip if no PNGs found
    filename=$(basename "$file")

    echo "Compressing: $filename"

    # Compress and write to output folder
    pngquant --quality=60-80 --force --output "$OUTPUT_DIR/$filename" "$file"
done

echo "Done! Compressed files are in: $OUTPUT_DIR"
