use ext_php_rs::args::Arg;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::{DnfTerm, PhpType, Zval};
use ext_php_rs::zend::ExecuteData;
use ext_php_rs::zend_fastcall;

fn dnf_a_and_b_or_c() -> PhpType {
    PhpType::Dnf(vec![
        DnfTerm::Intersection(vec!["DnfA".to_owned(), "DnfB".to_owned()]),
        DnfTerm::Single("DnfC".to_owned()),
    ])
}

fn dnf_two_intersections() -> PhpType {
    PhpType::Dnf(vec![
        DnfTerm::Intersection(vec!["DnfA".to_owned(), "DnfB".to_owned()]),
        DnfTerm::Intersection(vec!["DnfA".to_owned(), "DnfD".to_owned()]),
    ])
}

zend_fastcall! {
    extern "C" fn handler_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
        let mut arg = Arg::new("value", dnf_a_and_b_or_c());
        if execute_data.parser().arg(&mut arg).parse().is_err() {
            return;
        }
        retval.set_long(1);
    }
}

zend_fastcall! {
    extern "C" fn handler_nullable_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
        let mut arg = Arg::new("value", dnf_a_and_b_or_c()).allow_null();
        if execute_data.parser().arg(&mut arg).parse().is_err() {
            return;
        }
        retval.set_long(1);
    }
}

zend_fastcall! {
    extern "C" fn handler_two_intersections_arg(execute_data: &mut ExecuteData, retval: &mut Zval) {
        let mut arg = Arg::new("value", dnf_two_intersections());
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
    let arg_fn = FunctionBuilder::new("test_dnf_arg", handler_arg)
        .arg(Arg::new("value", dnf_a_and_b_or_c()))
        .returns(DataType::Long, false, false);

    let nullable_arg_fn = FunctionBuilder::new("test_dnf_nullable_arg", handler_nullable_arg)
        .arg(Arg::new("value", dnf_a_and_b_or_c()).allow_null())
        .returns(DataType::Long, false, false);

    let two_intersections_arg_fn = FunctionBuilder::new(
        "test_dnf_two_intersections_arg",
        handler_two_intersections_arg,
    )
    .arg(Arg::new("value", dnf_two_intersections()))
    .returns(DataType::Long, false, false);

    let returns_fn = FunctionBuilder::new("test_dnf_returns", handler_returns).returns(
        dnf_a_and_b_or_c(),
        false,
        false,
    );

    let nullable_returns_fn = FunctionBuilder::new("test_dnf_nullable_returns", handler_returns)
        .returns(dnf_a_and_b_or_c(), false, true);

    builder
        .function(arg_fn)
        .function(nullable_arg_fn)
        .function(two_intersections_arg_fn)
        .function(returns_fn)
        .function(nullable_returns_fn)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dnf_metadata_matches_reflection() {
        assert!(crate::integration::test::run_php("dnf/dnf.php"));
    }
}
