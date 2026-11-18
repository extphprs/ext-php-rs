<?php

// Properties declared with `ClassBuilder::typed_property` carry their types,
// as `protected int $count = 0;` does in a PHP class.
$class = new ReflectionClass(TestTypedProperties::class);

$count = $class->getProperty('count');
assert($count->isProtected());
assert($count->hasType());
assert((string) $count->getType() === 'int', (string) $count->getType());
assert($count->hasDefaultValue() && $count->getDefaultValue() === 0);

$label = $class->getProperty('label');
assert((string) $label->getType() === '?string', (string) $label->getType());
assert($label->getType()->allowsNull());
assert(!$label->hasDefaultValue(), 'a typed property without a default is uninitialized');

$items = $class->getProperty('items');
assert((string) $items->getType() === 'ArrayObject', (string) $items->getType());

$ratio = $class->getProperty('ratio');
assert((string) $ratio->getType() === 'float');
assert($ratio->getDefaultValue() === 1.5);

// An int default on a float property is a float, as PHP converts it.
$scale = $class->getProperty('scale');
assert($scale->getDefaultValue() === 2.0, var_export($scale->getDefaultValue(), true));

$object = new TestTypedProperties();
assert($object->scale === 2.0, var_export($object->scale, true));

// Uninitialized until written, as in PHP.
assert(!isset($object->label));
try {
    $object->label;
    assert(false, 'reading an uninitialized typed property must throw');
} catch (Error $e) {
    assert(str_contains($e->getMessage(), 'must not be accessed before initialization'), $e->getMessage());
}

$object->label = 'hello';
assert($object->label === 'hello');
$object->label = null;
assert($object->label === null);

// The engine enforces the type on writes.
try {
    $object->items = 'not an ArrayObject';
    assert(false, 'a value of another type must be refused');
} catch (TypeError $e) {
    assert(str_contains($e->getMessage(), 'ArrayObject'), $e->getMessage());
}
$object->items = new ArrayObject([1, 2]);
assert(count($object->items) === 2);

// Weak mode coerces as for a PHP typed property.
$object->ratio = 2;
assert($object->ratio === 2.0);

unset($object->label);
assert(!isset($object->label));

// A subclass reads and writes the protected property with its type.
class TypedPropertiesChild extends TestTypedProperties
{
    public function bump(): int
    {
        return ++$this->count;
    }

    public function setCount(mixed $value): void
    {
        $this->count = $value;
    }
}

$child = new TypedPropertiesChild();
assert($child->bump() === 1);
try {
    $child->setCount([]);
    assert(false, 'an array is not an int');
} catch (TypeError $e) {
    assert(str_contains($e->getMessage(), 'int'), $e->getMessage());
}
