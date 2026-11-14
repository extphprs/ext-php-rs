use ext_php_rs::prelude::*;

#[php_function]
#[php(sensitive(pasword))]
pub fn login(user: String, password: String) -> bool {
    !user.is_empty() && !password.is_empty()
}

fn main() {}
