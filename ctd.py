#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""Compile Typst document
ctd.py 2024 ckunte

Usage: ctd.py (-f <file>)
       ctd.py --help
       ctd.py --version

Options:
  -h, --help  Show this help
  -f, --file   Specify Typst input file to compile (required)

"""
import typst
from docopt import docopt

def main(typstfile):
    pdffile = f"{typstfile.rsplit('.', 1)[0]}.pdf"
    print(f"Compiling {typstfile} to {pdffile}...", end="")
    typst.compile(typstfile, output=pdffile)
    print("done.")

if __name__ == '__main__':
    args = docopt(__doc__, version="Compile Typst document, 0.1")
    main(args["<file>"])