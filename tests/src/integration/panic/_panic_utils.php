<?php

function expect_rust_panic(callable $callback, string $needle): void
{
    try {
        $callback();
    } catch (\Exception $e) {
        assert(false, 'a Rust panic must not be catchable as \Exception: ' . get_class($e));
    } catch (\Error $e) {
        assert(get_class($e) === \Error::class, get_class($e));
        assert(str_starts_with($e->getMessage(), 'Rust panic: '), $e->getMessage());
        assert(str_contains($e->getMessage(), $needle), $e->getMessage());
        return;
    }
    assert(false, 'no Error was thrown');
}
