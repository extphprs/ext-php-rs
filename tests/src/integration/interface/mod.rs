use ext_php_rs::prelude::*;
use ext_php_rs::types::{ZendClassObject, Zval};
use ext_php_rs::zend::ce;

#[php_interface]
#[php(extends(ce = ce::throwable, stub = "\\Throwable"))]
#[php(name = "ExtPhpRs\\Interface\\EmptyObjectInterface")]
#[allow(dead_code)]
pub trait EmptyObjectTrait {
    const STRING_CONST: &'static str = "STRING_CONST";

    const USIZE_CONST: u64 = 200;

    fn void();

    fn non_static(&self, data: String) -> String;

    fn ref_to_like_this_class(
        &self,
        data: String,
        other: &ZendClassObject<PhpInterfaceEmptyObjectTrait>,
    ) -> String;

    #[php(defaults(value = 0))]
    fn set_value(&mut self, value: i32);
}

// ============================================================================
// Test Feature 3: Implementing PHP's built-in Iterator interface
// This addresses GitHub issue #308 - Iterator from Rust
// ============================================================================

/// A simple range iterator that demonstrates implementing PHP's Iterator
/// interface. This allows the class to be used with PHP's foreach loop.
///
/// Usage in PHP:
/// ```php
/// $range = new RangeIterator(1, 5);
/// foreach ($range as $key => $value) {
///     echo "$key => $value\n";
/// }
/// // Output:
/// // 0 => 1
/// // 1 => 2
/// // 2 => 3
/// // 3 => 4
/// // 4 => 5
/// ```
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\RangeIterator")]
#[php(implements(ce = ce::iterator, stub = "\\Iterator"))]
pub struct RangeIterator {
    start: i64,
    end: i64,
    current: i64,
    index: i64,
}

#[php_impl]
impl RangeIterator {
    /// Create a new range iterator from start to end (inclusive).
    pub fn __construct(start: i64, end: i64) -> Self {
        Self {
            start,
            end,
            current: start,
            index: 0,
        }
    }

    /// Return the current element.
    /// PHP Iterator interface method.
    pub fn current(&self) -> i64 {
        self.current
    }

    /// Return the key of the current element.
    /// PHP Iterator interface method.
    pub fn key(&self) -> i64 {
        self.index
    }

    /// Move forward to next element.
    /// PHP Iterator interface method.
    pub fn next(&mut self) {
        self.current += 1;
        self.index += 1;
    }

    /// Rewind the Iterator to the first element.
    /// PHP Iterator interface method.
    pub fn rewind(&mut self) {
        self.current = self.start;
        self.index = 0;
    }

    /// Checks if current position is valid.
    /// PHP Iterator interface method.
    pub fn valid(&self) -> bool {
        self.current <= self.end
    }
}

/// An iterator over string key-value pairs to demonstrate mixed types.
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\MapIterator")]
#[php(implements(ce = ce::iterator, stub = "\\Iterator"))]
pub struct MapIterator {
    keys: Vec<String>,
    values: Vec<String>,
    index: usize,
}

#[php_impl]
impl MapIterator {
    /// Create a new map iterator with predefined data.
    pub fn __construct() -> Self {
        Self {
            keys: vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string(),
            ],
            values: vec!["one".to_string(), "two".to_string(), "three".to_string()],
            index: 0,
        }
    }

    /// Return the current element.
    pub fn current(&self) -> Option<String> {
        self.values.get(self.index).cloned()
    }

    /// Return the key of the current element.
    pub fn key(&self) -> Option<String> {
        self.keys.get(self.index).cloned()
    }

    /// Move forward to next element.
    pub fn next(&mut self) {
        self.index += 1;
    }

    /// Rewind the Iterator to the first element.
    pub fn rewind(&mut self) {
        self.index = 0;
    }

    /// Checks if current position is valid.
    pub fn valid(&self) -> bool {
        self.index < self.keys.len()
    }
}

/// An iterator that wraps a Rust Vec and exposes it to PHP.
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\VecIterator")]
#[php(implements(ce = ce::iterator, stub = "\\Iterator"))]
pub struct VecIterator {
    items: Vec<i64>,
    index: usize,
}

#[php_impl]
impl VecIterator {
    /// Create a new empty vec iterator.
    pub fn __construct() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
        }
    }

    /// Add an item to the iterator.
    pub fn push(&mut self, item: i64) {
        self.items.push(item);
    }

    /// Return the current element.
    pub fn current(&self) -> Option<i64> {
        self.items.get(self.index).copied()
    }

    /// Return the key of the current element.
    pub fn key(&self) -> usize {
        self.index
    }

    /// Move forward to next element.
    pub fn next(&mut self) {
        self.index += 1;
    }

    /// Rewind the Iterator to the first element.
    pub fn rewind(&mut self) {
        self.index = 0;
    }

    /// Checks if current position is valid.
    pub fn valid(&self) -> bool {
        self.index < self.items.len()
    }

    /// Get the number of items.
    pub fn count(&self) -> usize {
        self.items.len()
    }
}

// Note: Cross-crate interface discovery is now supported via the `inventory` crate.
// You can use `#[php_impl_interface]` to implement interfaces defined in other crates.
// See the `php_interface` and `php_impl_interface` macros for more details.

// ============================================================================
// Test Feature 5: Short form implements syntax
// Using #[php(implements("\\InterfaceName"))] instead of the verbose
// #[php(implements(ce = ce::interface, stub = "\\InterfaceName"))]
// ============================================================================

/// Test class implementing `ArrayAccess` using short form syntax.
/// This tests runtime class entry lookup via `ClassEntry::try_find_no_autoload()`.
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\ShortFormArrayAccess")]
#[php(implements("\\ArrayAccess"))]
pub struct ShortFormArrayAccess {
    data: Vec<i64>,
}

#[php_impl]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
impl ShortFormArrayAccess {
    pub fn __construct() -> Self {
        Self {
            data: vec![10, 20, 30, 40, 50],
        }
    }

    /// `ArrayAccess::offsetExists` - offset must be `mixed` to match PHP interface.
    pub fn offset_exists(&self, offset: &Zval) -> bool {
        if let Some(idx) = offset.long() {
            idx >= 0 && (idx as usize) < self.data.len()
        } else {
            false
        }
    }

    /// `ArrayAccess::offsetGet` - offset must be `mixed` to match PHP interface.
    pub fn offset_get(&self, offset: &Zval) -> Option<i64> {
        let idx = offset.long()?;
        if idx >= 0 {
            self.data.get(idx as usize).copied()
        } else {
            None
        }
    }

    /// `ArrayAccess::offsetSet` - offset and value must be `mixed` to match PHP interface.
    pub fn offset_set(&mut self, offset: &Zval, value: &Zval) {
        if let (Some(idx), Some(val)) = (offset.long(), value.long())
            && idx >= 0
            && (idx as usize) < self.data.len()
        {
            self.data[idx as usize] = val;
        }
    }

    /// `ArrayAccess::offsetUnset` - offset must be `mixed` to match PHP interface.
    pub fn offset_unset(&mut self, offset: &Zval) {
        if let Some(idx) = offset.long()
            && idx >= 0
            && (idx as usize) < self.data.len()
        {
            self.data.remove(idx as usize);
        }
    }
}

/// Test class implementing `Countable` using short form syntax.
/// `Countable` is defined in SPL which is always available.
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\CountableTest")]
#[php(implements("\\Countable"))]
pub struct CountableTest {
    items: Vec<String>,
}

#[php_impl]
#[allow(clippy::cast_possible_wrap)]
impl CountableTest {
    pub fn __construct() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: String) {
        self.items.push(item);
    }

    /// Returns the count for `count()`.
    pub fn count(&self) -> i64 {
        self.items.len() as i64
    }
}

/// Test class implementing multiple interfaces using mixed syntax
/// (both short form and explicit form).
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\MixedImplementsTest")]
#[php(implements(ce = ce::iterator, stub = "\\Iterator"))]
#[php(implements("\\Countable"))]
pub struct MixedImplementsTest {
    items: Vec<i64>,
    index: usize,
}

#[php_impl]
impl MixedImplementsTest {
    pub fn __construct() -> Self {
        Self {
            items: vec![10, 20, 30],
            index: 0,
        }
    }

    // Iterator methods
    pub fn current(&self) -> Option<i64> {
        self.items.get(self.index).copied()
    }

    pub fn key(&self) -> usize {
        self.index
    }

    pub fn next(&mut self) {
        self.index += 1;
    }

    pub fn rewind(&mut self) {
        self.index = 0;
    }

    pub fn valid(&self) -> bool {
        self.index < self.items.len()
    }

    #[allow(clippy::cast_possible_wrap)]
    // Countable method
    pub fn count(&self) -> i64 {
        self.items.len() as i64
    }
}

// Test Feature 2: Interface inheritance via trait bounds
// Define a parent interface
#[php_interface]
#[php(name = "ExtPhpRs\\Interface\\ParentInterface")]
#[allow(dead_code)]
pub trait ParentInterface {
    fn parent_method(&self) -> String;
}

// Define a child interface that extends the parent via Rust trait bounds
#[php_interface]
#[php(name = "ExtPhpRs\\Interface\\ChildInterface")]
#[allow(dead_code)]
pub trait ChildInterface: ParentInterface {
    fn child_method(&self) -> String;
}

// ============================================================================
// Test Feature 4: Using #[php_impl_interface] with inventory-based discovery
// This demonstrates cross-crate interface discovery via the inventory crate.
// ============================================================================

/// A simple greeter class that implements the `ParentInterface` via
/// `#[php_impl_interface]`.
#[php_class]
#[php(name = "ExtPhpRs\\Interface\\Greeter")]
pub struct Greeter {
    name: String,
}

#[php_impl]
impl Greeter {
    pub fn __construct(name: String) -> Self {
        Self { name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[php_impl_interface]
impl ParentInterface for Greeter {
    fn parent_method(&self) -> String {
        format!("Hello from {}!", self.name)
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .interface::<PhpInterfaceEmptyObjectTrait>()
        .interface::<PhpInterfaceParentInterface>()
        .interface::<PhpInterfaceChildInterface>()
        // Iterator examples for issue #308
        .class::<RangeIterator>()
        .class::<MapIterator>()
        .class::<VecIterator>()
        // Greeter with #[php_impl_interface]
        .class::<Greeter>()
        // Short form implements syntax tests
        .class::<ShortFormArrayAccess>()
        .class::<CountableTest>()
        .class::<MixedImplementsTest>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn interface_work() {
        assert!(crate::integration::test::run_php("interface/interface.php"));
    }
}
