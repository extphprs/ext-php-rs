use ext_php_rs::{boxed::ZBox, prelude::*, types::ZendHashTable, zend::ProcessGlobals};

#[php_function]
pub fn test_globals_http_get() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_get_vars().unwrap().to_owned()
}

#[php_function]
pub fn test_globals_http_post() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_post_vars().unwrap().to_owned()
}

#[php_function]
pub fn test_globals_http_cookie() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_cookie_vars().unwrap().to_owned()
}

#[php_function]
pub fn test_globals_http_server() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_server_vars().unwrap().to_owned()
}

#[php_function]
pub fn test_globals_http_request() -> ZBox<ZendHashTable> {
    ProcessGlobals::get()
        .http_request_vars()
        .unwrap()
        .unwrap()
        .to_owned()
}

#[php_function]
pub fn test_globals_http_files() -> ZBox<ZendHashTable> {
    ProcessGlobals::get().http_files_vars().unwrap().to_owned()
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_globals_http_get))
        .function(wrap_function!(test_globals_http_post))
        .function(wrap_function!(test_globals_http_cookie))
        .function(wrap_function!(test_globals_http_server))
        .function(wrap_function!(test_globals_http_request))
        .function(wrap_function!(test_globals_http_files))
}

#[cfg(test)]
mod tests {
    #[test]
    fn globals_works() {
        assert!(crate::integration::test::run_php("globals/globals.php"));
    }
}
