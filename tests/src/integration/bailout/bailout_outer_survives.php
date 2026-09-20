<?php

bailout_test_reset();

register_shutdown_function(function () {
    $counter = bailout_test_get_counter();
    if ($counter !== 1) {
        fwrite(STDERR, "Expected 1 destructors, got {$counter}\n");
        exit(1);
    }
});

bailout_test_outer_guard(function () {
    bailout_test_trigger();
});

fwrite(STDERR, "unreachable: the bailout did not propagate\n");
exit(1);
