<?php

require __DIR__ . '/../_utils.php';

// Signed
assert(test_number_signed(-12) === -12);
assert(test_number_signed(0) === 0);
assert(test_number_signed(12) === 12);

// Unsigned
assert(test_number_unsigned(0) === 0);
assert(test_number_unsigned(12) === 12);
assert_exception_thrown(fn() => test_number_unsigned(-12));

// Float
assert(round(test_number_float(-1.2), 2) === round(-1.2, 2));
assert(round(test_number_float(0.0), 2) === round(0.0, 2));
assert(round(test_number_float(1.2), 2) === round(1.2, 2));
