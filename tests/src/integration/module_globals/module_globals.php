<?php

// GINIT should have been called during module init
assert(test_module_globals_ginit_called() === true);

// ginit sets max_depth to 512
assert(test_module_globals_get_max_depth() === 512);

// counter starts at 0 (Default)
assert(test_module_globals_get_counter() === 0);

// increment and read back
test_module_globals_increment_counter();
assert(test_module_globals_get_counter() === 1);

test_module_globals_increment_counter();
test_module_globals_increment_counter();
assert(test_module_globals_get_counter() === 3);

// reset and verify
test_module_globals_reset_counter();
assert(test_module_globals_get_counter() === 0);
