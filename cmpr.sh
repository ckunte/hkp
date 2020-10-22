#!/usr/bin/env bash
# Compress all PDF files in the current folder
echo -n "Compressing all pdf files in this folder..."
for file in *.pdf; do
    gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -dPDFSETTINGS=/ebook -sOutputFile="compressed_${file}" "${file}"
done
echo "done."
echo "Compressed file: compressed_${file}."
