<?php

function heap_growth(callable $read): int
{
    for ($i = 0; $i < 16; $i++) {
        $read();
    }
    gc_collect_cycles();
    $before = memory_get_usage();
    for ($i = 0; $i < 500; $i++) {
        $read();
    }
    gc_collect_cycles();

    return memory_get_usage() - $before;
}

try {
    throw_exception_with_message_prop();
} catch (TestExceptionMessageLeak $e) {
}

assert($e->getMessage() === 'leak-bait message contents', var_export($e->getMessage(), true));

$reflection = new ReflectionProperty($e, 'message');
$reads = [
    'getMessage' => $e->getMessage(...),
    '__toString' => static fn() => (string) $e,
    'ReflectionProperty::getValue' => static fn() => $reflection->getValue($e)
];
foreach ($reads as $name => $read) {
    $growth = heap_growth($read);
    assert($growth < 8192, "{$name} on a #[php(prop)] String field grew the heap by {$growth} bytes over 500 calls");
}

$e->setMessage('changed');
assert($e->getMessage() === 'changed');
assert($e->message === 'changed');
