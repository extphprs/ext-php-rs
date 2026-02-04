use crate::ffi::{ZEND_MM_ALIGNMENT, ZEND_MM_ALIGNMENT_MASK, zend_execute_data};

use crate::{
    args::{ArgParser, ArgZvals},
    class::RegisteredClass,
    types::{ZendClassObject, ZendObject, Zval},
};

use super::function::Function;

/// Execute data passed when a function is called from PHP.
///
/// This generally contains things related to the call, including but not
/// limited to:
///
/// * Arguments
/// * `$this` object reference
/// * Reference to return value
/// * Previous execute data
pub type ExecuteData = zend_execute_data;

impl ExecuteData {
    /// Returns an [`ArgParser`] pre-loaded with the arguments contained inside
    /// `self`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData, args::Arg, flags::DataType};
    ///
    /// #[unsafe(no_mangle)]
    /// pub extern "C" fn example_fn(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     let mut a = Arg::new("a", DataType::Long);
    ///
    ///     // The `parse_args!()` macro can be used for this.
    ///     let parser = ex.parser()
    ///         .arg(&mut a)
    ///         .parse();
    ///
    ///     if parser.is_err() {
    ///         return;
    ///     }
    ///
    ///     dbg!(a);
    /// }
    /// ```
    pub fn parser(&mut self) -> ArgParser<'_, '_> {
        self.parser_object().0
    }

    /// Returns an [`ArgParser`] pre-loaded with the arguments contained inside
    /// `self`.
    ///
    /// A reference to `$this` is also returned in an [`Option`], which resolves
    /// to [`None`] if this function is not called inside a method.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData, args::Arg, flags::DataType};
    ///
    /// #[unsafe(no_mangle)]
    /// pub extern "C" fn example_fn(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     let mut a = Arg::new("a", DataType::Long);
    ///
    ///     let (parser, this) = ex.parser_object();
    ///     let parser = parser
    ///         .arg(&mut a)
    ///         .parse();
    ///
    ///     if parser.is_err() {
    ///         return;
    ///     }
    ///
    ///     dbg!(a, this);
    /// }
    /// ```
    pub fn parser_object(&mut self) -> (ArgParser<'_, '_>, Option<&mut ZendObject>) {
        // SAFETY: All fields of the `u2` union are the same type.
        let n_args = unsafe { self.This.u2.num_args };

        // Use stack-based storage for <= 8 arguments (the common case)
        // This avoids heap allocation for most function calls
        let args = ArgZvals::from_iter((0..n_args).map(|i| {
            // SAFETY: Function definition ensures arg lifetime doesn't exceed execution
            // data lifetime.
            unsafe { self.zend_call_arg(i as usize) }
        }));

        let obj = self.This.object_mut();

        (ArgParser::from_arg_zvals(args), obj)
    }

    /// Returns an [`ArgParser`] pre-loaded with the arguments contained inside
    /// `self`.
    ///
    /// A reference to `$this` is also returned in an [`Option`], which resolves
    /// to [`None`] if this function is not called inside a method.
    ///
    /// This function differs from [`parse_object`] in the fact that it returns
    /// a reference to a [`ZendClassObject`], which is an object that
    /// contains an arbitrary Rust type at the start of the object. The
    /// object will also resolve to [`None`] if the function is called
    /// inside a method that does not belong to an object with type `T`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData, args::Arg, flags::DataType, prelude::*};
    ///
    /// #[php_class]
    /// #[derive(Debug)]
    /// struct Example;
    ///
    /// #[unsafe(no_mangle)]
    /// pub extern "C" fn example_fn(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     let mut a = Arg::new("a", DataType::Long);
    ///
    ///     let (parser, this) = ex.parser_method::<Example>();
    ///     let parser = parser
    ///         .arg(&mut a)
    ///         .parse();
    ///
    ///     if parser.is_err() {
    ///         return;
    ///     }
    ///
    ///     dbg!(a, this);
    /// }
    ///
    /// #[php_module]
    /// pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    ///     module
    /// }
    /// ```
    ///
    /// [`parse_object`]: #method.parse_object
    pub fn parser_method<T: RegisteredClass>(
        &mut self,
    ) -> (ArgParser<'_, '_>, Option<&mut ZendClassObject<T>>) {
        let (parser, obj) = self.parser_object();
        (
            parser,
            obj.and_then(|obj| ZendClassObject::from_zend_obj_mut(obj)),
        )
    }

    /// Attempts to retrieve a reference to the underlying class object of the
    /// Zend object.
    ///
    /// Returns a [`ZendClassObject`] if the execution data contained a valid
    /// object of type `T`, otherwise returns [`None`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData, prelude::*};
    ///
    /// #[php_class]
    /// #[derive(Debug)]
    /// struct Example;
    ///
    /// #[unsafe(no_mangle)]
    /// pub extern "C" fn example_fn(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     let this = ex.get_object::<Example>();
    ///     dbg!(this);
    /// }
    ///
    /// #[php_module]
    /// pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    ///     module
    /// }
    /// ```
    pub fn get_object<T: RegisteredClass>(&mut self) -> Option<&mut ZendClassObject<T>> {
        ZendClassObject::from_zend_obj_mut(self.get_self()?)
    }

    /// Attempts to retrieve the 'this' object, even if the Rust backing
    /// is not yet initialized. This is used internally by the constructor
    /// to get access to the object before calling `initialize()`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the returned object is not dereferenced
    /// to `T` until after `initialize()` is called. Only `initialize()` should
    /// be called on the returned object.
    pub(crate) fn get_object_uninit<T: RegisteredClass>(
        &mut self,
    ) -> Option<&mut ZendClassObject<T>> {
        ZendClassObject::from_zend_obj_mut_uninit(self.get_self()?)
    }

    /// Attempts to retrieve the 'this' object, which can be used in class
    /// methods to retrieve the underlying Zend object.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData};
    ///
    /// #[unsafe(no_mangle)]
    /// pub extern "C" fn example_fn(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     let this = ex.get_self();
    ///     dbg!(this);
    /// }
    /// ```
    pub fn get_self(&mut self) -> Option<&mut ZendObject> {
        // TODO(david): This should be a `&mut self` function but we need to fix arg
        // parser first.
        self.This.object_mut()
    }

    /// Attempt to retrieve the function that is being called.
    #[must_use]
    pub fn function(&self) -> Option<&Function> {
        unsafe { self.func.as_ref() }
    }

    /// Attempt to retrieve the previous execute data on the call stack.
    #[must_use]
    pub fn previous(&self) -> Option<&Self> {
        unsafe { self.prev_execute_data.as_ref() }
    }

    /// Returns the number of arguments passed to this function call.
    ///
    /// This is useful for raw functions that need to know how many arguments
    /// were passed without using the full argument parser.
    #[inline]
    #[must_use]
    pub fn num_args(&self) -> u32 {
        // SAFETY: All fields of the `u2` union are the same type.
        unsafe { self.This.u2.num_args }
    }

    /// Gets a reference to the argument at the given index (0-based).
    ///
    /// This is a low-level method for raw function implementations that need
    /// direct access to arguments without type conversion overhead. For most
    /// use cases, prefer using the [`ArgParser`] via [`parser()`].
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `n` is less than [`num_args()`]
    /// - The returned reference is not held longer than the function call
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::{types::Zval, zend::ExecuteData};
    ///
    /// fn raw_handler(ex: &mut ExecuteData, retval: &mut Zval) {
    ///     if ex.num_args() >= 1 {
    ///         if let Some(arg) = unsafe { ex.get_arg(0) } {
    ///             if let Some(n) = arg.long() {
    ///                 retval.set_long(n + 1);
    ///                 return;
    ///             }
    ///         }
    ///     }
    ///     retval.set_null();
    /// }
    /// ```
    ///
    /// [`parser()`]: #method.parser
    /// [`num_args()`]: #method.num_args
    #[inline]
    #[must_use]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_arg<'a>(&self, n: usize) -> Option<&'a mut Zval> {
        // SAFETY: Caller must ensure n < num_args() and reference is not held
        // longer than the function call. The lifetime 'a is unbound from &self
        // following the same pattern as zend_call_arg() for PHP internal reasons.
        unsafe { self.zend_call_arg(n) }
    }

    /// Translation of macro `ZEND_CALL_ARG(call, n)`
    /// zend_compile.h:578
    ///
    /// The resultant [`Zval`] reference has a lifetime equal to the lifetime of
    /// `self`. This isn't specified because when you attempt to get a
    /// reference to args and the `$this` object, Rust doesn't let you.
    /// Since this is a private method it's up to the caller to ensure the
    /// lifetime isn't exceeded.
    #[doc(hidden)]
    #[inline]
    unsafe fn zend_call_arg<'a>(&self, n: usize) -> Option<&'a mut Zval> {
        let n = isize::try_from(n).expect("n is too large");
        let ptr = unsafe { self.zend_call_var_num(n) };
        unsafe { ptr.as_mut() }
    }

    /// Translation of macro `ZEND_CALL_VAR_NUM(call, n)`
    /// zend_compile.h: 575
    #[doc(hidden)]
    unsafe fn zend_call_var_num(&self, n: isize) -> *mut Zval {
        let ptr = std::ptr::from_ref(self) as *mut Zval;
        unsafe { ptr.offset(Self::zend_call_frame_slot() + n) }
    }

    /// Translation of macro `ZEND_CALL_FRAME_SLOT`
    /// zend_compile:573
    #[doc(hidden)]
    fn zend_call_frame_slot() -> isize {
        (Self::zend_mm_aligned_size::<Self>() + Self::zend_mm_aligned_size::<Zval>() - 1)
            / Self::zend_mm_aligned_size::<Zval>()
    }

    /// Translation of macro `ZEND_MM_ALIGNED_SIZE(size)`
    /// zend_alloc.h:41
    #[doc(hidden)]
    fn zend_mm_aligned_size<T>() -> isize {
        let size = isize::try_from(std::mem::size_of::<T>()).expect("size of T is too large");
        (size + ZEND_MM_ALIGNMENT - 1) & ZEND_MM_ALIGNMENT_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::ExecuteData;

    #[test]
    fn test_zend_call_frame_slot() {
        // PHP 8.0.2 (cli) (built: Feb 21 2021 11:51:33) ( NTS )
        // Copyright (c) The PHP Group
        // Zend Engine v4.0.2, Copyright (c) Zend Technologies
        assert_eq!(ExecuteData::zend_call_frame_slot(), 5);
    }

    // Note: num_args() and get_arg() tests require a real PHP execution context
    // and are tested indirectly through the raw function benchmarks and
    // integration tests. The methods are simple wrappers around PHP
    // internals and the implementation is straightforward enough that unit
    // testing the edge cases isn't practical without a full PHP execution
    // context.
}
