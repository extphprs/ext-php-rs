use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{PhpType, Zval};
use ext_php_rs::zend::ExecuteData;
use ext_php_rs::zend_fastcall;

#[php_class]
pub struct ClassUnionLeft;

#[php_class]
pub struct ClassUnionRight;

fn class_union() -> PhpType {
    PhpType::ClassUnion(vec![
        "ClassUnionLeft".to_owned(),
        "ClassUnionRight".to_owned(),
    ])
}

zend_fastcall! {
    extern "C" fn handler_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
        let mut arg = Arg::new("value", class_union());
        if execute_data.parser().arg(&mut arg).parse().is_err() {
            return;
        }
        retval.set_long(1);
    }
}

zend_fastcall! {
    extern "C" fn handler_nullable_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
        let mut arg = Arg::new("value", class_union()).allow_null();
        if execute_data.parser().arg(&mut arg).parse().is_err() {
            return;
        }
        retval.set_long(1);
    }
}

zend_fastcall! {
    extern "C" fn handler_returns(execute_data: &mut ExecuteData, retval: &mut Zval) {
        if execute_data.parser().parse().is_err() {
            return;
        }
        retval.set_null();
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let arg_fn = FunctionBuilder::new("test_class_union_arg", handler_arg)
        .arg(Arg::new("value", class_union()))
        .returns(DataType::Long, false, false);

    let nullable_arg_fn =
        FunctionBuilder::new("test_class_union_nullable_arg", handler_nullable_arg)
            .arg(Arg::new("value", class_union()).allow_null())
            .returns(DataType::Long, false, false);

    let returns_fn = FunctionBuilder::new("test_class_union_returns", handler_returns).returns(
        class_union(),
        false,
        false,
    );

    let nullable_returns_fn = FunctionBuilder::new(
        "test_class_union_nullable_returns",
        handler_returns,
    )
    .returns(class_union(), false, true);

    builder
        .function(arg_fn)
        .function(nullable_arg_fn)
        .function(returns_fn)
        .function(nullable_returns_fn)
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_union_metadata_matches_reflection() {
        assert!(crate::integration::test::run_php(
            "class_union/class_union.php"
        ));
    }
}
