<?php

$obj = new stdClass();
$obj->string = 'string';
$obj->bool = true;
$obj->number = 2022;
$obj->array = [
    1,
    2,
    3
];

$test = test_object($obj);

assert($test->string === 'string');
assert($test->bool === true);
assert($test->number === 2022);
assert($test->array === [1, 2, 3]);

assert('string' === test_object_read_property($obj, 'string'));

try {
    test_object_read_property($obj, 'missing');
    assert(false, 'reading a missing property should have failed');
} catch (\Throwable $e) {
    assert('property `missing` does not exist on the object' === $e->getMessage(), $e->getMessage());
}
