use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{PhpType, Zval};
use ext_php_rs::zend::ExecuteData;

extern "C" fn handler_int_or_string(execute_data: &mut ExecuteData, retval: &mut Zval) {
    let mut arg = Arg::new(
        "value",
        PhpType::Union(vec![DataType::Long, DataType::String]),
    );

    let parser = execute_data.parser().arg(&mut arg).parse();
    if parser.is_err() {
        return;
    }

    let Some(zval) = arg.zval() else {
        retval.set_long(0);
        return;
    };

    if zval.is_long() {
        retval.set_long(1);
    } else if zval.is_string() {
        retval.set_long(2);
    } else {
        retval.set_long(0);
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let f = FunctionBuilder::new("test_union_int_or_string", handler_int_or_string)
        .arg(Arg::new(
            "value",
            PhpType::Union(vec![DataType::Long, DataType::String]),
        ))
        .returns(DataType::Long, false, false);
    builder.function(f)
}

#[cfg(test)]
mod tests {
    #[test]
    fn union_int_or_string_works() {
        assert!(crate::integration::test::run_php("union/union.php"));
    }
}
