//! Types and traits for adding properties to PHP classes registered from Rust.
//!
//! There are two types of properties:
//!
//! * Field properties, referencing a property on a struct.
//! * Method properties, a getter and/or setter function called to get/set the
//!   value of the property.
//!
//! Field types which can be used as a property implement [`Prop`]. This is
//! automatically implemented on any type which implements [`Clone`],
//! [`IntoZval`] and [`FromZval`].
//!
//! Method property types only have to implement [`IntoZval`] for setters and
//! [`FromZval`] for getters.
//!
//! Properties are stored in the [`Property`] type, which allows us to store
//! field and method properties in one data structure. Properties are usually
//! retrieved via the [`RegisteredClass`] trait.
//!
//! [`RegisteredClass`]: crate::class::RegisteredClass

use crate::{
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    exception::PhpResult,
    types::Zval,
};

/// Implemented on types which can be used as PHP properties.
///
/// Generally, this should not be directly implemented on types, as it is
/// automatically implemented on types that implement [`Clone`], [`IntoZval`]
/// and [`FromZval`], which will be required to implement this trait regardless.
pub trait Prop<'a> {
    /// Gets the value of `self` by setting the value of `zv`.
    ///
    /// # Parameters
    ///
    /// * `zv` - The zval to set the value of.
    ///
    /// # Errors
    ///
    /// If the property could not be read.
    fn get(&self, zv: &mut Zval) -> Result<()>;

    /// Sets the value of `self` with the contents of a given zval `zv`.
    ///
    /// # Parameters
    ///
    /// * `zv` - The zval containing the new value of `self`.
    ///
    /// # Errors
    ///
    /// If the property could not be set
    fn set(&mut self, zv: &'a Zval) -> Result<()>;
}

impl<'a, T: Clone + IntoZval + FromZval<'a>> Prop<'a> for T {
    fn get(&self, zv: &mut Zval) -> Result<()> {
        self.clone().set_zval(zv, false)
    }

    fn set(&mut self, zv: &'a Zval) -> Result<()> {
        let x = Self::from_zval(zv).ok_or_else(|| Error::ZvalConversion(zv.get_type()))?;
        *self = x;
        Ok(())
    }
}

/// A getter for a property
pub type PropertyGetter<'a, T> = Option<Box<dyn Fn(&T, &mut Zval) -> PhpResult + Send + Sync + 'a>>;
/// A setter for a property
pub type PropertySetter<'a, T> = Option<Box<dyn Fn(&mut T, &Zval) -> PhpResult + Send + Sync + 'a>>;

/// Represents a property added to a PHP class.
pub enum Property<'a, T> {
    /// Field properties, where the data is stored inside a struct field.
    Field(Box<dyn (Fn(&mut T) -> &mut dyn Prop) + Send + Sync>),
    /// Method properties, where getter and/or setter functions are provided,
    /// which are used to get and set the value of the property.
    Method {
        /// Getter function for the property.
        get: PropertyGetter<'a, T>,
        /// Setter function for the property.
        set: PropertySetter<'a, T>,
    },
}

impl<'a, T: 'a> Property<'a, T> {
    /// Creates a field property.
    ///
    /// # Parameters
    ///
    /// * `f` - The function used to get a mutable reference to the property.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// struct Test {
    ///     pub a: i32,
    /// }
    ///
    /// let prop: Property<Test> = Property::field(|test: &mut Test| &mut test.a);
    /// ```
    pub fn field<F>(f: F) -> Self
    where
        F: (Fn(&mut T) -> &mut dyn Prop) + Send + Sync + 'static,
    {
        Self::Field(Box::new(f) as Box<dyn (Fn(&mut T) -> &mut dyn Prop) + Send + Sync>)
    }

    /// Creates a method property with getters and setters.
    ///
    /// If either the getter or setter is not given, an exception will be thrown
    /// when attempting to retrieve/set the property.
    ///
    /// # Parameters
    ///
    /// * `get` - Function used to get the value of the property, in an
    ///   [`Option`].
    /// * `set` - Function used to set the value of the property, in an
    ///   [`Option`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// struct Test;
    ///
    /// impl Test {
    ///     pub fn get_prop(&self) -> String {
    ///         "Hello".into()
    ///     }
    ///
    ///     pub fn set_prop(&mut self, val: String) {
    ///         println!("{}", val);
    ///     }
    /// }
    ///
    /// let prop: Property<Test> = Property::method(Some(Test::get_prop), Some(Test::set_prop));
    /// ```
    #[must_use]
    pub fn method<V>(get: Option<fn(&T) -> V>, set: Option<fn(&mut T, V)>) -> Self
    where
        for<'b> V: IntoZval + FromZval<'b> + 'a,
    {
        let get = get.map(|get| {
            Box::new(move |self_: &T, retval: &mut Zval| -> PhpResult {
                let value = get(self_);
                value
                    .set_zval(retval, false)
                    .map_err(|e| format!("Failed to return property value to PHP: {e:?}"))?;
                Ok(())
            }) as Box<dyn Fn(&T, &mut Zval) -> PhpResult + Send + Sync + 'a>
        });

        let set = set.map(|set| {
            Box::new(move |self_: &mut T, value: &Zval| -> PhpResult {
                let val = V::from_zval(value)
                    .ok_or("Unable to convert property value into required type.")?;
                set(self_, val);
                Ok(())
            }) as Box<dyn Fn(&mut T, &Zval) -> PhpResult + Send + Sync + 'a>
        });

        Self::Method { get, set }
    }

    /// Creates a getter-only method property.
    ///
    /// This is useful when the getter returns a type that only implements
    /// [`IntoZval`] but not [`FromZval`] for all lifetimes, such as
    /// `&'static str`.
    ///
    /// # Parameters
    ///
    /// * `get` - Function used to get the value of the property.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// struct Test;
    ///
    /// impl Test {
    ///     pub fn get_prop(&self) -> &'static str {
    ///         "Hello"
    ///     }
    /// }
    ///
    /// let prop: Property<Test> = Property::method_getter(Test::get_prop);
    /// ```
    #[must_use]
    pub fn method_getter<V>(get: fn(&T) -> V) -> Self
    where
        V: IntoZval + 'a,
    {
        let get = Box::new(move |self_: &T, retval: &mut Zval| -> PhpResult {
            let value = get(self_);
            value
                .set_zval(retval, false)
                .map_err(|e| format!("Failed to return property value to PHP: {e:?}"))?;
            Ok(())
        }) as Box<dyn Fn(&T, &mut Zval) -> PhpResult + Send + Sync + 'a>;

        Self::Method {
            get: Some(get),
            set: None,
        }
    }

    /// Creates a setter-only method property.
    ///
    /// This is useful when the setter accepts a type that only implements
    /// [`FromZval`] but not [`IntoZval`].
    ///
    /// # Parameters
    ///
    /// * `set` - Function used to set the value of the property.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// struct Test {
    ///     value: String,
    /// }
    ///
    /// impl Test {
    ///     pub fn set_prop(&mut self, val: String) {
    ///         self.value = val;
    ///     }
    /// }
    ///
    /// let prop: Property<Test> = Property::method_setter(Test::set_prop);
    /// ```
    #[must_use]
    pub fn method_setter<V>(set: fn(&mut T, V)) -> Self
    where
        for<'b> V: FromZval<'b> + 'a,
    {
        let set = Box::new(move |self_: &mut T, value: &Zval| -> PhpResult {
            let val = V::from_zval(value)
                .ok_or("Unable to convert property value into required type.")?;
            set(self_, val);
            Ok(())
        }) as Box<dyn Fn(&mut T, &Zval) -> PhpResult + Send + Sync + 'a>;

        Self::Method {
            get: None,
            set: Some(set),
        }
    }

    /// Adds a setter to an existing getter-only property, or combines with
    /// another property.
    ///
    /// # Parameters
    ///
    /// * `other` - Another property to combine with this one. If `other` has a
    ///   getter and this property doesn't, the getter from `other` is used. If
    ///   `other` has a setter and this property doesn't, the setter from
    ///   `other` is used.
    ///
    /// # Returns
    ///
    /// A new property with the combined getters and setters.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Property::Method { get: g1, set: s1 }, Property::Method { get: g2, set: s2 }) => {
                Property::Method {
                    get: g1.or(g2),
                    set: s1.or(s2),
                }
            }
            // If either is a field property, prefer the method property
            (Property::Field(_), other @ Property::Method { .. }) => other,
            // If first is method or both are fields, prefer the first one
            (this @ (Property::Method { .. } | Property::Field(_)), Property::Field(_)) => this,
        }
    }

    /// Attempts to retrieve the value of the property from the given object
    /// `self_`.
    ///
    /// The value of the property, if successfully retrieved, is loaded into the
    /// given [`Zval`] `retval`. If unsuccessful, a [`PhpException`] is
    /// returned inside the error variant of a result.
    ///
    /// # Parameters
    ///
    /// * `self_` - The object to retrieve the property from.
    /// * `retval` - The [`Zval`] to load the value of the property into.
    ///
    /// # Returns
    ///
    /// Nothing upon success
    ///
    /// # Errors
    ///
    /// A [`PhpException`] inside an error variant when the property could not
    /// be retrieved.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// # use ext_php_rs::types::Zval;
    /// struct Test {
    ///     pub a: i32,
    /// }
    ///
    /// let prop: Property<Test> = Property::field(|obj: &mut Test| &mut obj.a);
    ///
    /// let mut test = Test { a: 500 };
    /// let mut zv = Zval::new();
    /// prop.get(&mut test, &mut zv).unwrap();
    /// assert_eq!(zv.long(), Some(500));
    /// ```
    ///
    /// [`PhpException`]: crate::exception::PhpException
    pub fn get(&self, self_: &'a mut T, retval: &mut Zval) -> PhpResult {
        match self {
            Property::Field(field) => field(self_)
                .get(retval)
                .map_err(|e| format!("Failed to get property value: {e:?}").into()),
            Property::Method { get, set: _ } => match get {
                Some(get) => get(self_, retval),
                None => Err("No getter available for this property.".into()),
            },
        }
    }

    /// Attempts to set the value of the property inside the given object
    /// `self_`.
    ///
    /// The new value of the property is supplied inside the given [`Zval`]
    /// `value`. If unsuccessful, a [`PhpException`] is returned inside the
    /// error variant of a result.
    ///
    /// # Parameters
    ///
    /// * `self_` - The object to set the property in.
    /// * `value` - The [`Zval`] containing the new content for the property.
    ///
    /// # Returns
    ///
    /// Nothing upon success
    ///
    /// # Errors
    ///
    /// A [`PhpException`] inside an error variant when the property could not
    /// be set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use ext_php_rs::props::Property;
    /// # use ext_php_rs::types::Zval;
    /// # use ext_php_rs::convert::IntoZval;
    /// struct Test {
    ///     pub a: i32,
    /// }
    ///
    /// let prop: Property<Test> = Property::field(|obj: &mut Test| &mut obj.a);
    ///
    /// let mut test = Test { a: 500 };
    /// let zv = 100.into_zval(false).unwrap();
    /// prop.set(&mut test, &zv).unwrap();
    /// assert_eq!(test.a, 100);
    /// ```
    ///
    /// [`PhpException`]: crate::exception::PhpException
    pub fn set(&self, self_: &'a mut T, value: &Zval) -> PhpResult {
        match self {
            Property::Field(field) => field(self_)
                .set(value)
                .map_err(|e| format!("Failed to set property value: {e:?}").into()),
            Property::Method { get: _, set } => match set {
                Some(set) => set(self_, value),
                None => Err("No setter available for this property.".into()),
            },
        }
    }
}
