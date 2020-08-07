#!/usr/bin/env bash
echo -n "Compressing all pdf files in this folder..."
for i in *.pdf; do gs -sDEVICE=pdfwrite -dCompatibilityLevel=1.4 -dPDFSETTINGS=/ebook -dNOPAUSE -dQUIET -dBATCH -sOutputFile=compr_$i $i; done
echo "done."