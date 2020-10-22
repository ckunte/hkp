#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
"""
Combine folder-wise PDF files using Ghostscript
2014 ckunte
"""

import os

curr_dir = os.getcwd()
folders = next(os.walk(curr_dir))[1]

for fldr_name in folders:
    pdf_files = os.path.join(fldr_name, "*.pdf")
    output_pdf = os.path.join(fldr_name, f"{fldr_name}-all.pdf")

    cmd = (
        f"gs -dBATCH -dNOPAUSE -sDEVICE=pdfwrite -sOUTPUTFILE={output_pdf} {pdf_files}"
    )

    os.system(cmd)
