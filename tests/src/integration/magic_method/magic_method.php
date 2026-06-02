<?php

$magicMethod = new MagicMethod();

// __set
$magicMethod->count = 10;
// __get
assert(10 === $magicMethod->count);
assert(null === $magicMethod->test);

//__isset
assert(true === isset($magicMethod->count));
assert(false === isset($magicMethod->noCount));

// __unset
unset($magicMethod->count);
assert(0 === $magicMethod->count);

// __toString
assert('0' === $magicMethod->__toString());
assert('0' === (string) $magicMethod);

// __invoke
assert(34 === $magicMethod(34));

// __debugInfo
$debug = print_r($magicMethod, true);
$expectedDebug = "MagicMethod Object\n(\n    [count] => 0\n)\n";
assert($expectedDebug === $debug);

// __call
assert('Hello' === $magicMethod->callMagicMethod(1, 2, 3));
assert(null === $magicMethod->callUndefinedMagicMethod());

// __call_static
assert('Hello from static call 1, 2, 3' === MagicMethod::callStaticSomeMagic(1, 2, 3));
assert(null === MagicMethod::callUndefinedStaticSomeMagic());
