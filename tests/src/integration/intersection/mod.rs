use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{PhpType, Zval};
use ext_php_rs::zend::ExecuteData;

fn intersection() -> PhpType {
    PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()])
}

extern "C" fn handler_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
    let mut arg = Arg::new("value", intersection());
    if execute_data.parser().arg(&mut arg).parse().is_err() {
        return;
    }
    retval.set_long(1);
}

extern "C" fn handler_returns(execute_data: &mut ExecuteData, retval: &mut Zval) {
    if execute_data.parser().parse().is_err() {
        return;
    }
    // Slice 03 only verifies metadata (Reflection); the actual return value
    // shape is exercised by separate object-handling tests.
    retval.set_null();
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let arg_fn = FunctionBuilder::new("test_intersection_arg", handler_arg)
        .arg(Arg::new("value", intersection()))
        .returns(DataType::Long, false, false);

    let returns_fn = FunctionBuilder::new("test_intersection_returns", handler_returns).returns(
        intersection(),
        false,
        false,
    );

    builder.function(arg_fn).function(returns_fn)
}

#[cfg(test)]
mod tests {
    #[test]
    fn intersection_metadata_matches_reflection() {
        assert!(crate::integration::test::run_php(
            "intersection/intersection.php"
        ));
    }
}
