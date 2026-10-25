use ext_php_rs::{
    prelude::*,
    types::{ZendClassObject, Zval},
};

#[php_class]
pub struct NestedHolder {
    groups: Vec<Vec<Zval>>,
}

#[php_impl]
impl NestedHolder {
    pub fn __construct() -> Self {
        Self {
            groups: vec![Vec::new(), Vec::new()],
        }
    }

    pub fn add(&mut self, group: usize, value: &Zval) {
        self.groups[group].push(value.shallow_clone());
    }

    pub fn reset(&mut self) {
        self.groups = Vec::new();
    }
}

#[php_class]
pub struct TwoFields {
    first: Vec<Zval>,
    second: Vec<Zval>,
}

#[php_impl]
impl TwoFields {
    pub fn __construct() -> Self {
        Self {
            first: Vec::new(),
            second: Vec::new(),
        }
    }

    pub fn add(&mut self, first: &Zval, second: &Zval) {
        self.first.push(first.shallow_clone());
        self.second.push(second.shallow_clone());
    }

    pub fn reset(&mut self) {
        *self = Self::__construct();
    }
}

#[php_class]
pub struct Reentrant {
    callback: Zval,
    count: i64,
}

#[php_impl]
impl Reentrant {
    pub fn __construct() -> Self {
        Self {
            callback: Zval::new(),
            count: 0,
        }
    }

    pub fn set_callback(&mut self, callback: &Zval) {
        self.callback = callback.shallow_clone();
    }

    pub fn call_while_modifying(&mut self) -> PhpResult<Zval> {
        self.count += 1;
        Ok(self.callback.try_call(vec![])?)
    }

    pub fn call_while_reading(&self) -> PhpResult<Zval> {
        Ok(self.callback.try_call(vec![])?)
    }

    pub fn bump(&mut self) -> i64 {
        self.count += 1;
        self.count
    }

    pub fn count(&self) -> i64 {
        self.count
    }

    pub fn absorb(&mut self, other: ClassRef<Reentrant>) -> i64 {
        self.count + other.count
    }
}

#[php_function]
pub fn payload_pair(first: ClassRefMut<Reentrant>, second: ClassRef<Reentrant>) -> i64 {
    first.count + second.count
}

#[php_class]
pub struct Emitter {
    listeners: Vec<Zval>,
}

#[php_impl]
impl Emitter {
    pub fn __construct() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn on(&mut self, listener: &Zval) {
        self.listeners.push(listener.shallow_clone());
    }

    pub fn count(&self) -> usize {
        self.listeners.len()
    }

    pub fn emit(self_: &ZendClassObject<Self>) -> PhpResult<()> {
        let listeners: Vec<Zval> = self_
            .try_borrow()?
            .listeners
            .iter()
            .map(Zval::shallow_clone)
            .collect();
        for listener in &listeners {
            listener.try_call(vec![])?;
        }
        Ok(())
    }
}

#[php_class]
pub struct CallsBackOnDrop {
    callback: Zval,
}

impl Drop for CallsBackOnDrop {
    fn drop(&mut self) {
        let _ = self.callback.try_call(vec![]);
    }
}

#[php_impl]
impl CallsBackOnDrop {
    pub fn __construct() -> Self {
        Self {
            callback: Zval::new(),
        }
    }

    pub fn set_callback(&mut self, callback: &Zval) {
        self.callback = callback.shallow_clone();
    }

    pub fn label(&self) -> String {
        if self.callback.is_null() {
            String::from("no callback")
        } else {
            String::from("alive")
        }
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<NestedHolder>()
        .class::<TwoFields>()
        .class::<Reentrant>()
        .class::<Emitter>()
        .class::<CallsBackOnDrop>()
        .function(wrap_function!(payload_pair))
}

#[cfg(test)]
mod tests {
    #[test]
    fn gc_never_reads_a_payload_during_its_drop() {
        assert!(crate::integration::test::run_php("payload/payload.php"));
    }
}
