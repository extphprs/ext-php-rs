<?php
/**
 * Demo: RustHotload::loadDir()
 *
 * This demonstrates loading a full Cargo project directory at runtime.
 *
 * Usage:
 *   php -dextension=../../target/release/libext_php_rs_hotload.dylib demo_loaddir.php
 */

echo "=== RustHotload::loadDir() Demo ===\n\n";

// Load the module from a Cargo project directory
echo "Loading my_module from directory...\n";
RustHotload::loadDir(__DIR__ . '/my_module', verbose: true);

echo "\n=== Testing Functions ===\n";

// Test the functions
echo "greet_user('World') = " . greet_user("World") . "\n";
echo "factorial(10) = " . factorial(10) . "\n";
echo "current_timestamp() = " . current_timestamp() . "\n";

echo "\n=== Loaded Modules ===\n";
foreach (RustHotload::list() as $module) {
    echo "  - $module\n";
}

echo "\n=== Done! ===\n";
