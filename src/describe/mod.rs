//! Types used to describe downstream extensions. Used by the `cargo-php`
//! CLI application to generate PHP stub files used by IDEs.
//!
//! The types live in `ext-php-rs-introspection` so `cargo-php` can read a
//! [`Description`] without linking the Zend engine. This module re-exports
//! them and converts the builders into them.
use std::vec::Vec as StdVec;

#[cfg(feature = "enum")]
use crate::builders::EnumBuilder;
use crate::{
    builders::{ClassBuilder, FunctionBuilder},
    constant::IntoConst,
    flags::{ClassFlags, MethodFlags, PropertyFlags},
    prelude::ModuleBuilder,
};

use ext_php_rs_introspection::abi::{Option, RString};
pub use ext_php_rs_introspection::*;

/// Builds a [`Module`] from a [`ModuleBuilder`].
/// This is used to generate the PHP stubs for the module.
impl From<ModuleBuilder<'_>> for Module {
    fn from(builder: ModuleBuilder) -> Self {
        let functions = builder.functions;

        // Include both classes and interfaces in the classes list.
        // Interfaces are distinguished by `Class::is_interface`.
        #[allow(unused_mut)]
        let mut classes = builder
            .interfaces
            .into_iter()
            .chain(builder.classes)
            .map(|c| c().into())
            .collect::<StdVec<_>>();

        #[cfg(feature = "closure")]
        classes.push(Class::closure());

        #[cfg(feature = "enum")]
        let enums = builder
            .enums
            .into_iter()
            .map(|e| e().into())
            .collect::<StdVec<_>>();
        #[cfg(not(feature = "enum"))]
        let enums = StdVec::new();

        Self {
            name: builder.name.into(),
            functions: functions
                .into_iter()
                .map(Function::from)
                .collect::<StdVec<_>>()
                .into(),
            classes: classes.into(),
            enums: enums.into(),
            constants: builder
                .constants
                .into_iter()
                .map(|(name, value, docs)| constant_from(name, &*value, docs))
                .collect::<StdVec<_>>()
                .into(),
        }
    }
}

impl From<FunctionBuilder<'_>> for Function {
    fn from(val: FunctionBuilder<'_>) -> Self {
        let ret_allow_null = val.ret_as_null;
        Function {
            name: val.name.into(),
            docs: DocBlock(
                val.docs
                    .iter()
                    .map(|d| (*d).into())
                    .collect::<StdVec<_>>()
                    .into(),
            ),
            ret: val
                .retval
                .map(|r| Retval {
                    ty: r,
                    nullable: r != DataType::Mixed && ret_allow_null,
                })
                .into(),
            params: val
                .args
                .into_iter()
                .map(Parameter::from)
                .collect::<StdVec<_>>()
                .into(),
        }
    }
}

impl From<ClassBuilder> for Class {
    fn from(val: ClassBuilder) -> Self {
        let is_interface =
            ClassFlags::from_bits_retain(val.get_flags()).contains(ClassFlags::Interface);
        Self {
            name: val.name.into(),
            docs: DocBlock(
                val.docs
                    .iter()
                    .map(|doc| (*doc).into())
                    .collect::<StdVec<_>>()
                    .into(),
            ),
            extends: val.extends.map(|(_, stub)| stub.into()).into(),
            implements: val
                .interfaces
                .into_iter()
                .map(|(_, stub)| stub.into())
                .collect::<StdVec<_>>()
                .into(),
            properties: val
                .properties
                .into_iter()
                .map(Property::from)
                .collect::<StdVec<_>>()
                .into(),
            methods: val
                .methods
                .into_iter()
                .map(|(builder, flags)| method_from(builder, flags))
                .collect::<StdVec<_>>()
                .into(),
            constants: val
                .constants
                .into_iter()
                .map(|(name, _, docs, stub)| Constant {
                    name: name.into(),
                    value: Option::Some(stub.into()),
                    docs: docs.into(),
                })
                .collect::<StdVec<_>>()
                .into(),
            is_interface,
        }
    }
}

#[cfg(feature = "enum")]
impl From<EnumBuilder> for Enum {
    fn from(val: EnumBuilder) -> Self {
        Self {
            name: val.name.into(),
            docs: DocBlock(
                val.docs
                    .iter()
                    .map(|d| (*d).into())
                    .collect::<StdVec<_>>()
                    .into(),
            ),
            cases: val
                .cases
                .into_iter()
                .map(EnumCase::from)
                .collect::<StdVec<_>>()
                .into(),
            backing_type: match val.datatype {
                DataType::Long => Some("int".into()),
                DataType::String => Some("string".into()),
                _ => None,
            }
            .into(),
        }
    }
}

#[cfg(feature = "enum")]
impl From<&'static crate::enum_::EnumCase> for EnumCase {
    fn from(val: &'static crate::enum_::EnumCase) -> Self {
        Self {
            name: val.name.into(),
            docs: DocBlock(
                val.docs
                    .iter()
                    .map(|d| (*d).into())
                    .collect::<StdVec<_>>()
                    .into(),
            ),
            value: val
                .discriminant
                .as_ref()
                .map(|v| match v {
                    crate::enum_::Discriminant::Int(i) => i.to_string().into(),
                    crate::enum_::Discriminant::String(s) => format!("'{s}'").into(),
                })
                .into(),
        }
    }
}

impl From<crate::builders::ClassProperty> for Property {
    fn from(val: crate::builders::ClassProperty) -> Self {
        let static_ = val.flags.contains(PropertyFlags::Static);
        let vis = Visibility::from(val.flags);
        let docs = val.docs.into();

        Self {
            name: val.name.into(),
            docs,
            ty: val.ty.into(),
            vis,
            static_,
            nullable: val.nullable,
            readonly: val.readonly,
            default: val.default_stub.map(RString::from).into(),
        }
    }
}

fn method_from(builder: FunctionBuilder<'_>, flags: MethodFlags) -> Method {
    let ret_allow_null = builder.ret_as_null;
    Method {
        name: builder.name.into(),
        docs: DocBlock(
            builder
                .docs
                .iter()
                .map(|d| (*d).into())
                .collect::<StdVec<_>>()
                .into(),
        ),
        retval: builder
            .retval
            .map(|r| Retval {
                ty: r,
                nullable: r != DataType::Mixed && ret_allow_null,
            })
            .into(),
        params: builder
            .args
            .into_iter()
            .map(Into::into)
            .collect::<StdVec<_>>()
            .into(),
        ty: flags.into(),
        r#static: flags.contains(MethodFlags::Static),
        visibility: flags.into(),
        r#abstract: flags.contains(MethodFlags::Abstract),
    }
}

impl From<MethodFlags> for MethodType {
    fn from(value: MethodFlags) -> Self {
        if value.contains(MethodFlags::IsConstructor) {
            return Self::Constructor;
        }
        if value.contains(MethodFlags::Static) {
            return Self::Static;
        }

        Self::Member
    }
}

impl From<PropertyFlags> for Visibility {
    fn from(value: PropertyFlags) -> Self {
        if value.contains(PropertyFlags::Protected) {
            return Self::Protected;
        }
        if value.contains(PropertyFlags::Private) {
            return Self::Private;
        }

        Self::Public
    }
}

impl From<MethodFlags> for Visibility {
    fn from(value: MethodFlags) -> Self {
        if value.contains(MethodFlags::Protected) {
            return Self::Protected;
        }

        if value.contains(MethodFlags::Private) {
            return Self::Private;
        }

        Self::Public
    }
}

fn constant_from(name: String, value: &dyn IntoConst, docs: DocComments) -> Constant {
    Constant {
        name: name.into(),
        value: Option::Some(value.stub_value().into()),
        docs: docs.into(),
    }
}

#[cfg(test)]
mod tests {
    #![cfg_attr(windows, feature(abi_vectorcall))]
    use cfg_if::cfg_if;

    use super::*;

    use crate::{args::Arg, test::test_function};

    #[test]
    fn test_module_from() {
        let builder = ModuleBuilder::new("test", "test_version")
            .function(FunctionBuilder::new("test_function", test_function));
        let module: Module = builder.into();
        assert_eq!(module.name, "test".into());
        assert_eq!(module.functions.len(), 1);
        cfg_if! {
            if #[cfg(feature = "closure")] {
                assert_eq!(module.classes.len(), 1);
            } else {
                assert_eq!(module.classes.len(), 0);
            }
        }
        assert_eq!(module.enums.len(), 0);
        assert_eq!(module.constants.len(), 0);
    }

    #[test]
    fn test_function_from() {
        let builder = FunctionBuilder::new("test_function", test_function)
            .docs(&["doc1", "doc2"])
            .arg(Arg::new("foo", DataType::Long))
            .returns(DataType::Bool, true, true);
        let function: Function = builder.into();
        assert_eq!(function.name, "test_function".into());
        assert_eq!(function.docs.0.len(), 2);
        assert_eq!(
            function.params,
            vec![Parameter {
                name: "foo".into(),
                ty: Option::Some(DataType::Long),
                nullable: false,
                variadic: false,
                default: Option::None,
            }]
            .into()
        );
        assert_eq!(
            function.ret,
            Option::Some(Retval {
                ty: DataType::Bool,
                nullable: true,
            })
        );
    }

    #[test]
    fn test_class_from() {
        let builder = ClassBuilder::new("TestClass")
            .docs(&["doc1", "doc2"])
            .extends((|| todo!(), "BaseClass"))
            .implements((|| todo!(), "Interface1"))
            .implements((|| todo!(), "Interface2"))
            .property(crate::builders::ClassProperty {
                name: "prop1".into(),
                flags: PropertyFlags::Public,
                default: None,
                docs: &["doc1"],
                ty: None,
                nullable: false,
                readonly: false,
                default_stub: None,
            })
            .method(
                FunctionBuilder::new("test_function", test_function),
                MethodFlags::Protected,
            );
        let class: Class = builder.into();

        assert_eq!(class.name, "TestClass".into());
        assert!(!class.is_interface);
        assert_eq!(class.docs.0.len(), 2);
        assert_eq!(class.extends, Option::Some("BaseClass".into()));
        assert_eq!(
            class.implements,
            vec!["Interface1".into(), "Interface2".into()].into()
        );
        assert_eq!(class.properties.len(), 1);
        assert_eq!(
            class.properties[0],
            Property {
                name: "prop1".into(),
                docs: DocBlock(vec!["doc1".into()].into()),
                ty: Option::None,
                vis: Visibility::Public,
                static_: false,
                nullable: false,
                readonly: false,
                default: Option::None,
            }
        );
        assert_eq!(class.methods.len(), 1);
        assert_eq!(
            class.methods[0],
            Method {
                name: "test_function".into(),
                docs: DocBlock(vec![].into()),
                ty: MethodType::Member,
                params: vec![].into(),
                retval: Option::None,
                r#static: false,
                visibility: Visibility::Protected,
                r#abstract: false
            }
        );
    }

    #[test]
    fn test_interface_from() {
        let class: Class = ClassBuilder::new("TestInterface")
            .flags(ClassFlags::Interface)
            .into();
        assert!(class.is_interface);
    }

    #[test]
    fn test_property_from() {
        let property: Property = crate::builders::ClassProperty {
            name: "test_property".into(),
            flags: PropertyFlags::Protected,
            default: None,
            docs: &["doc1", "doc2"],
            ty: Some(DataType::String),
            nullable: true,
            readonly: false,
            default_stub: Some("null".into()),
        }
        .into();
        assert_eq!(property.name, "test_property".into());
        assert_eq!(property.docs.0.len(), 2);
        assert_eq!(property.vis, Visibility::Protected);
        assert!(!property.static_);
        assert!(property.nullable);
        assert_eq!(property.default, Option::Some("null".into()));
        assert_eq!(property.ty, Option::Some(DataType::String));
    }

    #[test]
    fn test_method_from() {
        let builder = FunctionBuilder::new("test_method", test_function)
            .docs(&["doc1", "doc2"])
            .arg(Arg::new("foo", DataType::Long))
            .returns(DataType::Bool, true, true);
        let method = method_from(builder, MethodFlags::Static | MethodFlags::Protected);
        assert_eq!(method.name, "test_method".into());
        assert_eq!(method.docs.0.len(), 2);
        assert_eq!(
            method.params,
            vec![Parameter {
                name: "foo".into(),
                ty: Option::Some(DataType::Long),
                nullable: false,
                variadic: false,
                default: Option::None,
            }]
            .into()
        );
        assert_eq!(
            method.retval,
            Option::Some(Retval {
                ty: DataType::Bool,
                nullable: true,
            })
        );
        assert!(method.r#static);
        assert_eq!(method.visibility, Visibility::Protected);
        assert_eq!(method.ty, MethodType::Static);
    }

    #[test]
    fn test_ty_from() {
        let r#static: MethodType = MethodFlags::Static.into();
        assert_eq!(r#static, MethodType::Static);

        let constructor: MethodType = MethodFlags::IsConstructor.into();
        assert_eq!(constructor, MethodType::Constructor);

        let member: MethodType = MethodFlags::Public.into();
        assert_eq!(member, MethodType::Member);

        let mixed: MethodType = (MethodFlags::Protected | MethodFlags::Static).into();
        assert_eq!(mixed, MethodType::Static);

        let both: MethodType = (MethodFlags::Static | MethodFlags::IsConstructor).into();
        assert_eq!(both, MethodType::Constructor);

        let empty: MethodType = MethodFlags::empty().into();
        assert_eq!(empty, MethodType::Member);
    }

    #[test]
    fn test_prop_visibility_from() {
        let private: Visibility = PropertyFlags::Private.into();
        assert_eq!(private, Visibility::Private);

        let protected: Visibility = PropertyFlags::Protected.into();
        assert_eq!(protected, Visibility::Protected);

        let public: Visibility = PropertyFlags::Public.into();
        assert_eq!(public, Visibility::Public);

        let mixed: Visibility = (PropertyFlags::Protected | PropertyFlags::Static).into();
        assert_eq!(mixed, Visibility::Protected);

        let empty: Visibility = PropertyFlags::empty().into();
        assert_eq!(empty, Visibility::Public);
    }

    #[test]
    fn test_method_visibility_from() {
        let private: Visibility = MethodFlags::Private.into();
        assert_eq!(private, Visibility::Private);

        let protected: Visibility = MethodFlags::Protected.into();
        assert_eq!(protected, Visibility::Protected);

        let public: Visibility = MethodFlags::Public.into();
        assert_eq!(public, Visibility::Public);

        let mixed: Visibility = (MethodFlags::Protected | MethodFlags::Static).into();
        assert_eq!(mixed, Visibility::Protected);

        let empty: Visibility = MethodFlags::empty().into();
        assert_eq!(empty, Visibility::Public);
    }
}
