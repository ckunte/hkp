#!/usr/bin/env bash
echo -n "Combining all pdf files in the curr folder..."
gs -dBATCH -dNOPAUSE -q -sDEVICE=pdfwrite -sOUTPUTFILE=combined.pdf *.pdf;
echo "done."