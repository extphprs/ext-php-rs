<?php

assert(is_null(test_nullable()));
assert(!is_null(test_nullable('value')));

$param = ( new ReflectionFunction('test_nullable') )->getParameters()[0];
assert($param->allowsNull());
assert($param->isOptional());
assert($param->isDefaultValueAvailable());
assert($param->getDefaultValue() === null);
