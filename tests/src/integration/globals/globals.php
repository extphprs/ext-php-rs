<?php

assert(test_globals_http_get() === []);
assert(test_globals_http_post() === []);
assert(test_globals_http_cookie() === []);
assert(test_globals_http_server() !== []);
assert(test_globals_http_request() === []);
assert(test_globals_http_files() === []);
