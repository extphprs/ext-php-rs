<?php

declare(strict_types = 1);

$enum_variant = TestEnum::Variant1;
assert($enum_variant === TestEnum::Variant1);
assert($enum_variant !== TestEnum::Variant2);
assert(TestEnum::cases() === [TestEnum::Variant1, TestEnum::Variant2]);

assert(IntBackedEnum::Variant1->value === 1);
assert(IntBackedEnum::from(2) === IntBackedEnum::Variant2);
assert(IntBackedEnum::tryFrom(1) === IntBackedEnum::Variant1);
assert(IntBackedEnum::tryFrom(3) === null);

assert(StringBackedEnum::Variant1->value === 'foo');
assert(StringBackedEnum::from('bar') === StringBackedEnum::Variant2);
assert(StringBackedEnum::tryFrom('foo') === StringBackedEnum::Variant1);
assert(StringBackedEnum::tryFrom('baz') === null);

assert(test_enum(TestEnum::Variant1) === StringBackedEnum::Variant2);
