use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{PhpType, Zval};
use ext_php_rs::zend::ExecuteData;

/// Maps the parsed [`Zval`] to a small integer code so PHP-side assertions can
/// distinguish which union member was received without inspecting the value
/// itself: 1 = int, 2 = string, 3 = null, 0 = other / parse failure.
fn classify(zval: Option<&Zval>, retval: &mut Zval) {
    let code = match zval {
        Some(z) if z.is_null() => 3,
        Some(z) if z.is_long() => 1,
        Some(z) if z.is_string() => 2,
        _ => 0,
    };
    retval.set_long(code);
}

extern "C" fn handler_int_or_string(execute_data: &mut ExecuteData, retval: &mut Zval) {
    let mut arg = Arg::new(
        "value",
        PhpType::Union(vec![DataType::Long, DataType::String]),
    );
    if execute_data.parser().arg(&mut arg).parse().is_err() {
        return;
    }
    classify(arg.zval().map(|z| &**z), retval);
}

extern "C" fn handler_int_string_or_null(execute_data: &mut ExecuteData, retval: &mut Zval) {
    let mut arg = Arg::new(
        "value",
        PhpType::Union(vec![DataType::Long, DataType::String, DataType::Null]),
    );
    if execute_data.parser().arg(&mut arg).parse().is_err() {
        return;
    }
    classify(arg.zval().map(|z| &**z), retval);
}

extern "C" fn handler_int_string_allow_null(execute_data: &mut ExecuteData, retval: &mut Zval) {
    let mut arg = Arg::new(
        "value",
        PhpType::Union(vec![DataType::Long, DataType::String]),
    );
    if execute_data.parser().arg(&mut arg).parse().is_err() {
        return;
    }
    classify(arg.zval().map(|z| &**z), retval);
}

extern "C" fn handler_returns_int_or_string(execute_data: &mut ExecuteData, retval: &mut Zval) {
    if execute_data.parser().parse().is_err() {
        return;
    }
    retval.set_long(1);
}

extern "C" fn handler_returns_int_string_or_null(
    execute_data: &mut ExecuteData,
    retval: &mut Zval,
) {
    if execute_data.parser().parse().is_err() {
        return;
    }
    retval.set_null();
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let int_or_string = FunctionBuilder::new("test_union_int_or_string", handler_int_or_string)
        .arg(Arg::new(
            "value",
            PhpType::Union(vec![DataType::Long, DataType::String]),
        ))
        .returns(DataType::Long, false, false);

    let int_string_or_null = FunctionBuilder::new(
        "test_union_int_string_or_null",
        handler_int_string_or_null,
    )
    .arg(Arg::new(
        "value",
        PhpType::Union(vec![DataType::Long, DataType::String, DataType::Null]),
    ))
    .returns(DataType::Long, false, false);

    let int_string_allow_null = FunctionBuilder::new(
        "test_union_int_string_allow_null",
        handler_int_string_allow_null,
    )
    .arg(
        Arg::new(
            "value",
            PhpType::Union(vec![DataType::Long, DataType::String]),
        )
        .allow_null(),
    )
    .returns(DataType::Long, false, false);

    let returns_int_or_string = FunctionBuilder::new(
        "test_returns_int_or_string",
        handler_returns_int_or_string,
    )
    .returns(
        PhpType::Union(vec![DataType::Long, DataType::String]),
        false,
        false,
    );

    let returns_int_string_or_null = FunctionBuilder::new(
        "test_returns_int_string_or_null",
        handler_returns_int_string_or_null,
    )
    .returns(
        PhpType::Union(vec![DataType::Long, DataType::String, DataType::Null]),
        false,
        false,
    );

    builder
        .function(int_or_string)
        .function(int_string_or_null)
        .function(int_string_allow_null)
        .function(returns_int_or_string)
        .function(returns_int_string_or_null)
}

#[cfg(test)]
mod tests {
    #[test]
    fn union_int_or_string_works() {
        assert!(crate::integration::test::run_php("union/union.php"));
    }
}
