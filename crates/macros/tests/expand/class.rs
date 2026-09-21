#[macro_use]
extern crate ext_php_rs_derive;

/// Doc comments for MyClass.
/// This is a basic class example.
#[php_class]
pub struct MyClass {}

#[php_impl]
impl MyClass {
    #[php(getter)]
    pub fn get_first(&self) -> i64 {
        1
    }

    #[php(setter)]
    pub fn set_first(&mut self, _value: i64) {}

    #[php(getter)]
    pub fn get_second(&self) -> String {
        String::new()
    }

    pub fn plain(&self) {}
}
