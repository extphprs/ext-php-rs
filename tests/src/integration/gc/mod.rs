use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use ext_php_rs::{
    boxed::ZBox,
    prelude::*,
    types::{ZendObject, Zval},
};

static DROPPED: AtomicUsize = AtomicUsize::new(0);

#[php_class]
pub struct GcHolder {
    callback: Zval,
    listeners: Vec<Zval>,
    named: HashMap<String, Zval>,
    callable: Option<ZendCallable<'static>>,
    child: Option<ZBox<ZendObject>>,
    label: String,
}

impl Drop for GcHolder {
    fn drop(&mut self) {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

#[php_impl]
impl GcHolder {
    pub fn __construct() -> Self {
        Self {
            callback: Zval::new(),
            listeners: Vec::new(),
            named: HashMap::new(),
            callable: None,
            child: None,
            label: String::from("holder"),
        }
    }

    #[php(getter)]
    pub fn get_callback(&self) -> Zval {
        self.callback.shallow_clone()
    }

    pub fn set_callback(&mut self, callback: &Zval) {
        self.callback = callback.shallow_clone();
    }

    pub fn add_listener(&mut self, listener: &Zval) {
        self.listeners.push(listener.shallow_clone());
    }

    pub fn set_named(&mut self, name: String, listener: &Zval) {
        self.named.insert(name, listener.shallow_clone());
    }

    pub fn set_callable(&mut self, callable: &Zval) {
        self.callable = ZendCallable::new_owned(callable.shallow_clone()).ok();
    }

    pub fn child(&mut self) -> Zval {
        let child = self.child.get_or_insert_with(ZendObject::new_stdclass);
        let mut zval = Zval::new();
        zval.set_object(child);
        zval
    }

    pub fn label(&self) -> String {
        self.label.clone()
    }
}

#[php_function]
pub fn gc_holder_dropped() -> usize {
    DROPPED.load(Ordering::Relaxed)
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<GcHolder>()
        .function(wrap_function!(gc_holder_dropped))
}

#[cfg(test)]
mod tests {
    #[test]
    fn gc_collects_cycles_through_rust_fields() {
        assert!(crate::integration::test::run_php("gc/gc.php"));
    }
}
