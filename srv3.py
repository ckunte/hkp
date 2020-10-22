#!/usr/bin/env python
# http.server script for clean URLs
# Credits: R. Scott-Adams ( https://stackoverflow.com/a/28467686 )

import os
import http.server

class SuffixHandler(http.server.SimpleHTTPRequestHandler):
    """
    Overrides the default request handler to assume a suffixless 
    resource is actually an html page of the same name. Thus,
    http://localhost:8000/foo would find foo.html.
    """
    def do_GET(self):
        path = self.translate_path(self.path)
        # If the path doesn't exist, then assume it's a resource
        # suffixed with '.html':
        if not os.path.exists(path):
            self.path = self.path + '.html'
        # Call the superclass methods to actually serve the page:
        http.server.SimpleHTTPRequestHandler.do_GET(self)

http.server.test(SuffixHandler)