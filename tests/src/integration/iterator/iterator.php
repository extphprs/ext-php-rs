<?php

assert(iter_next([]) === []);
assert(iter_next([1, 2, 3]) === [0, 1, 1, 2, 2, 3]);
assert(iter_back([]) === []);
assert(iter_back([1, 2, 3]) === [2, 3, 1, 2, 0, 1]);

assert(iter_next_back([], 2) === [null, null]);
assert(iter_next_back([1, 2, 3], 2) === [2, 3, 0, 1, 1, 2, null, null]);
var_dump(iter_next_back([1, 2, 3, 4, 5], 3));
assert(iter_next_back([1, 2, 3, 4, 5], 3) === [4, 5, 0, 1, 1, 2, 3, 4, 2, 3, null, null, null]);
