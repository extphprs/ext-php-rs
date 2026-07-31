<?php

// The declared parameter type must be `iterable`. PHP 8.2 replaced the
// `1 << IS_ITERABLE` arginfo mask with a dedicated bit, so getting this wrong
// makes the engine reject every argument and trips an arginfo/zpp mismatch on
// debug builds.
$reflected = new ReflectionFunction('iterable_count');
$param = $reflected->getParameters()[0];
assert((string) $param->getType() === 'iterable', 'parameter should be declared iterable');

// Arrays reach Iterable through the Array variant.
assert(iterable_count([]) === 0);
assert(iterable_count([1, 2, 3]) === 3);
assert(iterable_keys_to_string(['a' => 1, 'b' => 2]) === 'a,b');
assert(iterable_values_to_string([10, 20, 30]) === '10,20,30');

// Generators reach Iterable through the Traversable variant, which is the
// variant that used to be produced by laundering a shared reference.
function gen(): Generator
{
    yield 'x' => 'first';
    yield 'y' => 'second';
}

assert(iterable_count(gen()) === 2);
assert(iterable_keys_to_string(gen()) === 'x,y');
assert(iterable_values_to_string(gen()) === 'first,second');

// An Iterator implementation is also Traversable.
class Counter implements Iterator
{
    private int $i = 0;

    public function current(): mixed
    {
        return $this->i * 10;
    }

    public function key(): mixed
    {
        return $this->i;
    }

    public function next(): void
    {
        $this->i++;
    }

    public function rewind(): void
    {
        $this->i = 0;
    }

    public function valid(): bool
    {
        return $this->i < 3;
    }
}

assert(iterable_count(new Counter()) === 3);
assert(iterable_keys_to_string(new Counter()) === '0,1,2');
assert(iterable_values_to_string(new Counter()) === '0,10,20');
