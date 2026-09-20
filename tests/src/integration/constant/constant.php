<?php

declare(strict_types = 1);

$reflection = new ReflectionClass(TestConstantVisibility::class);
assert($reflection->getReflectionConstant('PUBLIC_CONST')->isPublic());
assert($reflection->getReflectionConstant('EXPLICIT_PUBLIC_CONST')->isPublic());
assert($reflection->getReflectionConstant('PROTECTED_CONST')->isProtected());
assert($reflection->getReflectionConstant('PRIVATE_CONST')->isPrivate());

assert(TestConstantVisibility::PUBLIC_CONST === 1);
assert(TestConstantVisibility::EXPLICIT_PUBLIC_CONST === 2);
assert(TestConstantVisibility::readPrivate() === true);
assert($reflection->getConstant('PROTECTED_CONST') === 'protected');

try {
    TestConstantVisibility::PRIVATE_CONST;
    assert(false, 'a private constant must not be readable from outside the class');
} catch (Error $e) {
    assert(str_contains($e->getMessage(), 'Cannot access private constant'), $e->getMessage());
}

try {
    TestConstantVisibility::PROTECTED_CONST;
    assert(false, 'a protected constant must not be readable from outside the class');
} catch (Error $e) {
    assert(str_contains($e->getMessage(), 'Cannot access protected constant'), $e->getMessage());
}

final class ConstantChild extends TestConstantVisibility
{
    public static function protectedValue(): string
    {
        return self::PROTECTED_CONST;
    }
}

assert(ConstantChild::protectedValue() === 'protected');
