#!/usr/bin/env python3
# -*- coding: UTF-8 -*-
"""
Combine folder-wise PDF files using Ghostscript
2014 ckunte
"""

import os

current_directory = os.getcwd()
folder = next(os.walk(current_directory))[1]

for folder_name in folder:
    pdf_files = os.path.join(folder_name, "*.pdf")
    output_pdf = os.path.join(folder_name, f"{folder_name}-all.pdf")

    cmd = (
        f"gs -dBATCH -dNOPAUSE -sDEVICE=pdfwrite -sOUTPUTFILE={output_pdf} {pdf_files}"
    )

    os.system(cmd)
