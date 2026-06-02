<?php

const TYPES = [
    'test_str' => [['string'], 'string'],
    'test_string' => [['string'], 'string'],
    'test_bool' => [['bool'], 'bool'],
    'test_number_signed' => [['int'], 'int'],
    'test_number_unsigned' => [['int'], 'int'],
    'test_number_float' => [['float'], 'float'],
    'test_array' => [['array'], 'array'],
    'test_array' => [['array'], 'array'],
    'test_array_assoc' => [['array'], 'array'],
    'test_binary' => [['string'], 'string'],
    'test_nullable' => [['?string'], '?string'],
    'test_object' => [['object'], 'object'],
    'test_closure' => [[], 'RustClosure'],
    'test_closure_once' => [['string'], 'RustClosure'],
    'test_callable' => [['callable', 'string'], 'mixed']
];

function toStr(ReflectionNamedType|ReflectionUnionType|ReflectionIntersectionType|null $v): string
{
    if ($v === null) {
        return '<null>';
    }
    return match (true) {
        $v instanceof ReflectionNamedType => $v->allowsNull() && $v->getName() !== 'mixed'
            ? '?' . $v->getName()
            : $v->getName(),
        $v instanceof ReflectionUnionType => $v->getName(),
        $v instanceof ReflectionIntersectionType => $v->getName()
    };
}

foreach (TYPES as $func => [$args, $return]) {
    $f = new ReflectionFunction($func);
    $tReturn = toStr($f->getReturnType());
    assert($tReturn === $return, "Wrong return type of {$func}, expected {$return}, got {$tReturn}");
    foreach ($f->getParameters() as $idx => $param) {
        $tParam = toStr($param->getType());
        assert($tParam === $args[$idx], "Wrong arg type {$idx} of {$func}, expected {$args[$idx]}, got {$tParam}");
    }
}
