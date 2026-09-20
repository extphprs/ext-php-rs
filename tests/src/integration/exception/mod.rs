use ext_php_rs::{
    prelude::*,
    types::{ZendCallable, Zval},
    zend::ce,
};

#[php_class]
#[php(name = "Test\\TestException")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Debug)]
pub struct TestException;

#[php_function]
pub fn throw_custom_exception() -> PhpResult<i32> {
    Err(PhpException::from_class::<TestException>(
        "Not good custom!".into(),
    ))
}

#[php_function]
pub fn throw_default_exception() -> PhpResult<i32> {
    Err(PhpException::from_message("Not good!".into()))
}

#[php_function]
pub fn call_throwing_callable(call: ZendCallable) -> PhpResult<()> {
    call.try_call(vec![])?;

    Ok(())
}

#[php_function]
pub fn throw_over_pending_exception(call: ZendCallable) {
    let _ = call.try_call(vec![]);
    PhpException::from_message("second".into()).throw();
}

#[php_function]
pub fn throw_non_object() -> PhpResult<()> {
    ext_php_rs::exception::throw_object(Zval::new())?;

    Ok(())
}

#[php_function]
pub fn throw_interface_class() {
    PhpException::new("cannot instantiate".into(), 0, ce::throwable()).throw();
}

#[php_function]
pub fn throw_nul_message() {
    PhpException::from_message("before\0after".into()).throw();
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<TestException>()
        .function(wrap_function!(throw_default_exception))
        .function(wrap_function!(throw_custom_exception))
        .function(wrap_function!(call_throwing_callable))
        .function(wrap_function!(throw_over_pending_exception))
        .function(wrap_function!(throw_non_object))
        .function(wrap_function!(throw_interface_class))
        .function(wrap_function!(throw_nul_message))
}

#[cfg(test)]
mod tests {
    #[test]
    fn exception_works() {
        assert!(crate::integration::test::run_php("exception/exception.php"));
    }
}
