<?php

declare(strict_types=1);

// Reflection proves we wrote the correct nested `zend_type_list` metadata
// for DNF (Disjunctive Normal Form) types. PHP exposes DNF as a
// `ReflectionUnionType` whose `getTypes()` may include
// `ReflectionIntersectionType` instances for the `(A&B)` parts and
// `ReflectionNamedType` instances for single classes. Internal-function
// arg type enforcement is `#if ZEND_DEBUG` in php-src, so the assertions
// below are metadata-driven and mirror the slice 03 intersection harness.

interface DnfA {}
interface DnfB {}
interface DnfD {}
class DnfC implements DnfA, DnfB {}

/**
 * Extract a sorted shape of a DNF reflection type:
 * - intersection groups become arrays of class names (sorted)
 * - single classes become the class name string
 * - the outer list is sorted alphabetically by string repr to deduplicate
 *   ordering noise from PHP's reflection.
 *
 * @return array<int, array{intersection: array<int, string>}|string>
 */
function dnf_member_shapes(ReflectionType $type): array {
    if (!$type instanceof ReflectionUnionType) {
        return [];
    }
    $out = [];
    foreach ($type->getTypes() as $member) {
        if ($member instanceof ReflectionIntersectionType) {
            $names = array_map(
                static fn(ReflectionNamedType $t): string => $t->getName(),
                $member->getTypes(),
            );
            sort($names);
            $out[] = ['intersection' => $names];
        } elseif ($member instanceof ReflectionNamedType) {
            $name = $member->getName();
            // PHP normalises a `null` member of a union to a separate
            // ReflectionNamedType("null"); we keep it as a string so the
            // assertion can spot it.
            $out[] = $name;
        }
    }
    usort($out, static function ($a, $b): int {
        $ka = is_array($a) ? 'i:' . implode('&', $a['intersection']) : 's:' . $a;
        $kb = is_array($b) ? 'i:' . implode('&', $b['intersection']) : 's:' . $b;
        return strcmp($ka, $kb);
    });
    return $out;
}

// (DnfA&DnfB)|DnfC arg.
$rf = new ReflectionFunction('test_dnf_arg');
$params = $rf->getParameters();
assert(count($params) === 1, 'test_dnf_arg: expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'test_dnf_arg: expected ReflectionUnionType',
);
assert(
    $params[0]->allowsNull() === false,
    'test_dnf_arg: must not allow null',
);

$shapes = dnf_member_shapes($type);
assert(
    $shapes === [['intersection' => ['DnfA', 'DnfB']], 'DnfC'],
    'test_dnf_arg: expected (DnfA&DnfB)|DnfC, got ' . json_encode($shapes),
);

// (DnfA&DnfB)|DnfC|null arg via allow_null flag.
$rf = new ReflectionFunction('test_dnf_nullable_arg');
$type = $rf->getParameters()[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'test_dnf_nullable_arg: expected ReflectionUnionType',
);
assert(
    $rf->getParameters()[0]->allowsNull() === true,
    'test_dnf_nullable_arg: must allow null',
);

// (DnfA&DnfB)|(DnfA&DnfD) arg.
$rf = new ReflectionFunction('test_dnf_two_intersections_arg');
$type = $rf->getParameters()[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'test_dnf_two_intersections_arg: expected ReflectionUnionType',
);
$shapes = dnf_member_shapes($type);
assert(
    $shapes === [
        ['intersection' => ['DnfA', 'DnfB']],
        ['intersection' => ['DnfA', 'DnfD']],
    ],
    'test_dnf_two_intersections_arg: expected (DnfA&DnfB)|(DnfA&DnfD), got '
        . json_encode($shapes),
);

// (DnfA&DnfB)|DnfC return.
$rf = new ReflectionFunction('test_dnf_returns');
$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'test_dnf_returns: expected ReflectionUnionType return',
);
assert(
    $ret->allowsNull() === false,
    'test_dnf_returns: must not allow null',
);
$shapes = dnf_member_shapes($ret);
assert(
    $shapes === [['intersection' => ['DnfA', 'DnfB']], 'DnfC'],
    'test_dnf_returns: expected (DnfA&DnfB)|DnfC, got ' . json_encode($shapes),
);

// (DnfA&DnfB)|DnfC|null return via allow_null flag.
$rf = new ReflectionFunction('test_dnf_nullable_returns');
$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'test_dnf_nullable_returns: expected ReflectionUnionType return',
);
assert(
    $ret->allowsNull() === true,
    'test_dnf_nullable_returns: must allow null',
);

// Smoke test that a value satisfying the DNF can flow through the call.
// Internal-function arg type enforcement only triggers in ZEND_DEBUG
// builds, so the call returns whether or not the argument shape matches;
// the metadata assertions above are the load-bearing checks.
$obj = new DnfC();
assert(test_dnf_arg($obj) === 1, 'test_dnf_arg call must succeed');
assert(test_dnf_nullable_arg(null) === 1, 'nullable DNF arg accepts null');
