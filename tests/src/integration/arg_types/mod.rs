//! Argument and return types `#[php_function]` cannot declare: no type at all,
//! and unions of scalar types, built with `FunctionBuilder`.

use ext_php_rs::args::Arg;
use ext_php_rs::builders::{ClassBuilder, FunctionBuilder};
use ext_php_rs::flags::{DataType, MethodFlags};
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;

ext_php_rs::zend_fastcall! {
    extern fn untyped_echo(ex: &mut ExecuteData, retval: &mut Zval) {
        if let Some(value) = unsafe { ex.zend_call_arg(0) } {
            *retval = value.shallow_clone();
        }
    }
}

ext_php_rs::zend_fastcall! {
    extern fn number_twice(ex: &mut ExecuteData, retval: &mut Zval) {
        let Some(value) = (unsafe { ex.zend_call_arg(0) }) else {
            return;
        };
        if let Some(long) = value.long() {
            retval.set_long(long * 2);
        } else if let Some(double) = value.double() {
            retval.set_double(double * 2.0);
        }
    }
}

/// A class whose abstract methods are declared with an untyped parameter and
/// with `int|float`, so PHP's inheritance checks can be exercised.
#[php_class]
#[php(modifier = abstract_methods)]
#[php(flags = ext_php_rs::flags::ClassFlags::Abstract)]
pub struct TestArgTypes;

#[php_impl]
impl TestArgTypes {
    pub fn __construct() -> Self {
        Self
    }
}

fn abstract_methods(builder: ClassBuilder) -> ClassBuilder {
    let abstract_public = MethodFlags::Public | MethodFlags::Abstract;
    builder
        .method(
            FunctionBuilder::new_abstract("check").arg(Arg::untyped("value")),
            abstract_public,
        )
        .method(
            FunctionBuilder::new_abstract("number")
                .arg(Arg::new("value", DataType::Long).or(DataType::Double))
                .returns(DataType::Long, false, false)
                .or_returns(DataType::Double),
            abstract_public,
        )
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(
            FunctionBuilder::new("test_untyped_echo", untyped_echo).arg(Arg::untyped("value")),
        )
        .function(
            FunctionBuilder::new("test_number_twice", number_twice)
                .arg(Arg::new("value", DataType::Long).or(DataType::Double))
                .returns(DataType::Long, false, false)
                .or_returns(DataType::Double),
        )
        .function(
            FunctionBuilder::new("test_nullable_union", untyped_echo).arg(
                Arg::new("value", DataType::String)
                    .or(DataType::Long)
                    .or(DataType::Null),
            ),
        )
        .class::<TestArgTypes>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn untyped_and_union_arguments() {
        assert!(crate::integration::test::run_php("arg_types/arg_types.php"));
    }
}
