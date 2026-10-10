use ext_php_rs::prelude::*;
use ext_php_rs::zend::{SapiRequestInfo, set_header};

fn mark_request(info: &SapiRequestInfo) {
    if let Some(uri) = info.request_uri() {
        let _ = set_header(&format!("X-Ext-Php-Rs-Activate: {uri}"));
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder.sapi_activate_function(mark_request)
}
