<?php
/**
 * Demo: ext-php-rs-hotload
 *
 * This demonstrates hot-loading Rust code at runtime with full ext-php-rs capabilities.
 * Functions defined in Rust become available as native PHP functions!
 *
 * Usage:
 *   php -dextension=../../target/release/libext_php_rs_hotload.dylib demo_host.php
 */

echo "=== ext-php-rs-hotload Demo ===\n\n";

// Show cache directory
echo "Cache directory: " . RustHotload::cacheDir() . "\n\n";

// Load the math plugin
echo "Loading math plugin...\n";
RustHotload::loadFile(__DIR__ . '/plugin_math.rs', verbose: true);

// Load the hello plugin
echo "\nLoading hello plugin...\n";
RustHotload::loadFile(__DIR__ . '/plugin_hello.rs', verbose: true);

// Show loaded plugins
echo "\n=== Loaded Plugins ===\n";
foreach (RustHotload::list() as $plugin) {
    echo "  - $plugin\n";
}

// Use the math functions
echo "\n=== Math Functions (from Rust) ===\n";
echo "add(100, 200) = " . add(100, 200) . "\n";
echo "subtract(1000, 337) = " . subtract(1000, 337) . "\n";
echo "multiply(12, 12) = " . multiply(12, 12) . "\n";
echo "fibonacci(30) = " . fibonacci(30) . "\n";

// Use the hello functions
echo "\n=== Hello Functions (from Rust) ===\n";
echo hello("World") . "\n";
echo hello("PHP + Rust") . "\n";
echo "Current timestamp (nanos): " . rust_nanos() . "\n";
echo "Sum of squares (1 to 100): " . sum_of_squares(100) . "\n";

// Performance comparison - both using iterative algorithm
$n = 90; // max before u64 overflow
echo "\n=== Performance: Fibonacci($n) x 100000 iterations ===\n";

// PHP version (iterative - same algorithm as Rust)
function php_fib($n) {
    if ($n <= 1) return $n;
    $a = 0;
    $b = 1;
    for ($i = 2; $i <= $n; $i++) {
        $c = $a + $b;
        $a = $b;
        $b = $c;
    }
    return $b;
}

$iterations = 100000;

$start = microtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $result_php = php_fib($n);
}
$time_php = microtime(true) - $start;

$start = microtime(true);
for ($i = 0; $i < $iterations; $i++) {
    $result_rust = fibonacci($n);
}
$time_rust = microtime(true) - $start;

printf("PHP:  fib(%d) = %d  (%.6f seconds)\n", $n, $result_php, $time_php);
printf("Rust: fib(%d) = %d  (%.6f seconds)\n", $n, $result_rust, $time_rust);
printf("Rust is %.1fx faster!\n", $time_php / $time_rust);

// Load Rust code from string (global functions)
echo "\n=== Load from String (Global Functions) ===\n";

RustHotload::loadString('
    #[php_function]
    fn square(x: i64) -> i64 { x * x }

    #[php_function]
    fn cube(x: i64) -> i64 { x * x * x }
', verbose: true);

echo "square(5) = " . square(5) . "\n";
echo "cube(5) = " . cube(5) . "\n";

// Single callable functions with func()
echo "\n=== Callable Functions with func() ===\n";

$triple = RustHotload::func('(x: i64) -> i64', 'x * 3');
$add = RustHotload::func('(a: i64, b: i64) -> i64', 'a + b');
$greet = RustHotload::func('(name: String) -> String', 'format!("Hi, {}.", name)');

echo "\$triple(14) = " . $triple(14) . "\n";
echo "\$add(10, 32) = " . $add(10, 32) . "\n";
echo "\$greet('World') = " . $greet("World") . "\n";

// Rust classes with state and methods
echo "\n=== Rust Classes with State ===\n";

$Counter = RustHotload::class(
    'value: i64',
    '
    __construct(start: i64) { Self { value: start } }
    add(&mut self, b: i64) { self.value += b }
    get(&self) -> i64 { self.value }
    '
);

$c = $Counter(5);
echo "Counter created with value 5\n";
echo "get() = " . $c->get() . "\n";
$c->add(10);
echo "After add(10): get() = " . $c->get() . "\n";
$c->add(100);
echo "After add(100): get() = " . $c->get() . "\n";

echo "\n=== Done! ===\n";
