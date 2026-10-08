<?php

$handler = static function (): void {
    $failures = array_keys(array_filter([
        'class' => !class_exists('TestClass'),
        'interface' => !interface_exists('ExtPhpRs\Interface\EmptyObjectInterface'),
        'enum' => !enum_exists('IntBackedEnum') || IntBackedEnum::from(2) !== IntBackedEnum::Variant2,
        'closure' => test_closure()('works') !== 'works'
    ]));

    if ($failures !== []) {
        http_response_code(500);
        echo 'missing: ' . implode(', ', $failures) . "\n";

        return;
    }

    echo "ok\n";
};

while (frankenphp_handle_request($handler)) {
    gc_collect_cycles();
}
