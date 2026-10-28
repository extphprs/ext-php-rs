#![warn(clippy::needless_pass_by_value)]

use ext_php_rs::{prelude::*, types::Zval};

#[php_class]
pub struct ArgCounter {
    count: i64,
    label: String,
    callback: Zval,
}

#[php_impl]
impl ArgCounter {
    pub fn __construct(count: i64, label: String) -> Self {
        Self {
            count,
            label,
            callback: Zval::new(),
        }
    }

    pub fn count(&self) -> i64 {
        self.count
    }

    pub fn bump(&mut self) -> i64 {
        self.count += 1;
        self.count
    }

    pub fn set_callback(&mut self, callback: &Zval) {
        self.callback = callback.shallow_clone();
    }

    pub fn add(&mut self, other: &ArgCounter) -> i64 {
        self.count += other.count;
        self.count
    }

    pub fn give(&self, other: &mut ArgCounter) -> i64 {
        other.count += self.count;
        other.count
    }

    pub fn add_maybe(&self, other: Option<&ArgCounter>) -> i64 {
        other.map_or(self.count, |other| self.count + other.count)
    }

    pub fn give_maybe(&self, other: Option<&mut ArgCounter>) -> i64 {
        other.map_or(-1, |other| {
            other.count += self.count;
            other.count
        })
    }

    pub fn sum(first: &ArgCounter, second: &ArgCounter) -> i64 {
        first.count + second.count
    }

    pub fn run_callback_of(&self, other: &ArgCounter) -> PhpResult<i64> {
        other.callback.try_call(vec![])?;
        Ok(self.count)
    }

    pub fn bump_while_calling(&mut self, callback: &Zval) -> PhpResult<Zval> {
        self.count += 1;
        Ok(callback.try_call(vec![])?)
    }

    pub fn add_all(&mut self, first: &ArgCounter, rest: &[ClassRef<ArgCounter>]) -> i64 {
        self.count += first.count + rest.iter().map(|other| other.count).sum::<i64>();
        self.count
    }
}

#[php_class]
pub struct ArgView {
    count: i64,
}

#[php_impl]
impl ArgView {
    pub fn __construct(source: &ArgCounter) -> Self {
        Self {
            count: source.count,
        }
    }

    pub fn count(&self) -> i64 {
        self.count
    }

    #[php(setter)]
    pub fn set_source(&mut self, source: &ArgCounter) {
        self.count = source.count;
    }
}

#[php_interface]
#[php(name = "ExtPhpRs\\ClassArgument\\Peer")]
pub trait Peer {
    fn peer_total(&self, other: &ArgCounter) -> i64;
}

#[php_impl_interface]
impl Peer for ArgCounter {
    fn peer_total(&self, other: &ArgCounter) -> i64 {
        self.count + other.count
    }
}

#[php_function]
pub fn arg_counter_pair(first: &mut ArgCounter, second: &ArgCounter) -> i64 {
    first.count += second.count;
    first.count
}

#[php_function]
pub fn arg_counter_label(counter: &ArgCounter) -> &str {
    &counter.label
}

#[php_function]
pub fn arg_counter_read(counter: &ArgCounter, callback: &Zval) -> PhpResult<i64> {
    callback.try_call(vec![])?;
    Ok(counter.count)
}

#[php_function]
pub fn arg_counter_total(counters: &[ClassRef<ArgCounter>]) -> i64 {
    counters.iter().map(|counter| counter.count).sum()
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .interface::<PhpInterfacePeer>()
        .class::<ArgCounter>()
        .class::<ArgView>()
        .function(wrap_function!(arg_counter_pair))
        .function(wrap_function!(arg_counter_label))
        .function(wrap_function!(arg_counter_read))
        .function(wrap_function!(arg_counter_total))
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_arguments_borrow_for_the_call() {
        assert!(crate::integration::test::run_php(
            "class_argument/class_argument.php"
        ));
    }
}
