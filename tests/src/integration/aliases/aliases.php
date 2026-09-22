<?php

assert(test_alias_option() === -1);
assert(test_alias_option(null) === -1);
assert(test_alias_option(41) === 41);
$param = ( new ReflectionFunction('test_alias_option') )->getParameters()[0];
assert($param->allowsNull());
assert($param->isOptional());
assert((string) $param->getType() === '?int');

$params = ( new ReflectionFunction('test_alias_nullable_required') )->getParameters();
assert($params[0]->allowsNull());
assert(!$params[0]->isOptional());
assert(test_alias_nullable_required(null, 2) === 2);
assert(test_alias_nullable_required(1, 2) === 3);

assert(test_qualified_option() === 'nobody');
assert(test_qualified_option(null) === 'nobody');
assert(test_qualified_option('Ada') === 'Ada');
$param = ( new ReflectionFunction('test_qualified_option') )->getParameters()[0];
assert((string) $param->getType() === '?string');
assert($param->isOptional());

assert(test_qualified_variadic() === 0);
assert(test_qualified_variadic(1, 'a', []) === 3);
$param = ( new ReflectionFunction('test_qualified_variadic') )->getParameters()[0];
assert($param->isVariadic());

assert(test_typed_variadic() === 0);
assert(test_typed_variadic(1, 2, 3) === 6);
$param = ( new ReflectionFunction('test_typed_variadic') )->getParameters()[0];
assert($param->isVariadic());
assert((string) $param->getType() === 'int');

$x = 5;
test_alias_phpref($x);
assert($x === 6);
assert(( new ReflectionFunction('test_alias_phpref') )->getParameters()[0]->isPassedByReference());

assert(test_object_by_value(new AliasCounter()) === 1);
$counter = new AliasCounter();
test_object_by_value($counter);
assert(test_object_by_value($counter) === 2);
assert($counter->count === 2);
assert(!( new ReflectionFunction('test_object_by_value') )->getParameters()[0]->isPassedByReference());

$arr = [];
test_array_by_ref($arr);
assert($arr === [1]);
assert(( new ReflectionFunction('test_array_by_ref') )->getParameters()[0]->isPassedByReference());
