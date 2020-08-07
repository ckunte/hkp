#!/usr/bin/env python
# -*- coding: UTF-8 -*-
'''
combine folder-wise pdf files
2014 ckunte
'''
import os
import platform

platforms = os.walk('.').next()[1]
for platform in platforms:
    msg = 'echo "Processing ' + str(platform) + ' ..";'
    cdr = 'cd ' + str(platform) + ';'
    if platform.system() == 'Darwin':
        cmd = '/System/Library/Automator/Combine\ PDF\ Pages.action/Contents/Resources/join.py -o' + str(platform) + '-all.pdf *.pdf;'
    else:
        cmd = 'gs -dBATCH -dNOPAUSE -sDEVICE=pdfwrite -sOUTPUTFILE=' + str(platform) + '-all.pdf *.pdf;'
    os.system(msg + cdr + cmd)
pass