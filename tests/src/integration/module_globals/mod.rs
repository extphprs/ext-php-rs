use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};

#[derive(Default)]
struct TestModuleGlobals {
    counter: i64,
    max_depth: i32,
    ginit_called: bool,
}

impl ModuleGlobal for TestModuleGlobals {
    fn ginit(&mut self) {
        self.ginit_called = true;
        self.max_depth = 512;
    }
}

static TEST_GLOBALS: ModuleGlobals<TestModuleGlobals> = ModuleGlobals::new();

#[php_function]
pub fn test_module_globals_get_counter() -> i64 {
    TEST_GLOBALS.get().counter
}

#[php_function]
pub fn test_module_globals_increment_counter() {
    unsafe { TEST_GLOBALS.get_mut() }.counter += 1;
}

#[php_function]
pub fn test_module_globals_get_max_depth() -> i32 {
    TEST_GLOBALS.get().max_depth
}

#[php_function]
pub fn test_module_globals_ginit_called() -> bool {
    TEST_GLOBALS.get().ginit_called
}

#[php_function]
pub fn test_module_globals_reset_counter() {
    unsafe { TEST_GLOBALS.get_mut() }.counter = 0;
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .globals(&TEST_GLOBALS)
        .function(wrap_function!(test_module_globals_get_counter))
        .function(wrap_function!(test_module_globals_increment_counter))
        .function(wrap_function!(test_module_globals_get_max_depth))
        .function(wrap_function!(test_module_globals_ginit_called))
        .function(wrap_function!(test_module_globals_reset_counter))
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_globals_works() {
        assert!(crate::integration::test::run_php(
            "module_globals/module_globals.php"
        ));
    }
}
