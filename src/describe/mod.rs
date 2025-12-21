//! Types used to describe downstream extensions. Used by the `cargo-php`
//! CLI application to generate PHP stub files used by IDEs.
use std::vec::Vec as StdVec;

#[cfg(feature = "enum")]
use crate::builders::EnumBuilder;
use crate::{
    builders::{ClassBuilder, FunctionBuilder},
    constant::IntoConst,
    flags::{DataType, MethodFlags, PropertyFlags},
    prelude::ModuleBuilder,
};
use abi::{Option, RString, Str, Vec};

pub mod abi;
mod stub;

pub use stub::ToStub;

/// A slice of strings containing documentation comments.
pub type DocComments = &'static [&'static str];

/// Representation of the extension used to generate PHP stubs.
#[repr(C)]
pub struct Description {
    /// Extension description.
    pub module: Module,
    /// ext-php-rs version.
    pub version: &'static str,
}

impl Description {
    /// Creates a new description.
    ///
    /// # Parameters
    ///
    /// * `module` - The extension module representation.
    #[must_use]
    pub fn new(module: Module) -> Self {
        Self {
            module,
            version: crate::VERSION,
        }
    }
}

/// Represents a set of comments on an export.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DocBlock(pub Vec<Str>);

impl From<&'static [&'static str]> for DocBlock {
    fn from(val: &'static [&'static str]) -> Self {
        Self(
            val.iter()
                .map(|s| (*s).into())
                .collect::<StdVec<_>>()
                .into(),
        )
    }
}

/// Represents an extension containing a set of exports.
#[repr(C)]
pub struct Module {
    /// Name of the extension.
    pub name: RString,
    /// Functions exported by the extension.
    pub functions: Vec<Function>,
    /// Classes exported by the extension.
    pub classes: Vec<Class>,
    #[cfg(feature = "enum")]
    /// Enums exported by the extension.
    pub enums: Vec<Enum>,
    /// Constants exported by the extension.
    pub constants: Vec<Constant>,
}

/// Builds a [`Module`] from a [`ModuleBuilder`].
/// This is used to generate the PHP stubs for the module.
impl From<ModuleBuilder<'_>> for Module {
    fn from(builder: ModuleBuilder) -> Self {
        let functions = builder.functions;

        #[allow(unused_mut)]
        let mut classes = builder
            .classes
            .into_iter()
            .map(|c| c().into())
            .collect::<StdVec<_>>();

        #[cfg(feature = "closure")]
        classes.push(Class::closure());

        Self {
            name: builder.name.into(),
            functions: functions
                .into_iter()
                .map(Function::from)
                .collect::<StdVec<_>>()
                .into(),
            classes: classes.into(),
            constants: builder
                .constants
                .into_iter()
                .map(Constant::from)
                .collect::<StdVec<_>>()
                .into(),
            #[cfg(feature = "enum")]
            enums: builder
                .enums
                .into_iter()
                .map(|e| e().into())
                .collect::<StdVec<_>>()
                .into(),
        }
    }
}

/// Represents an exported function.
#[repr(C)]
pub struct Function {
    /// Name of the function.
    pub name: RString,
    /// Documentation comments for the function.
    pub docs: DocBlock,
    /// Return value of the function.
    pub ret: Option<Retval>,
    /// Parameters of the function.
    pub params: Vec<Parameter>,
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

/// Represents a parameter attached to an exported function or method.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Parameter {
    /// Name of the parameter.
    pub name: RString,
    /// Type of the parameter.
    pub ty: Option<DataType>,
    /// Whether the parameter is nullable.
    pub nullable: bool,
    /// Whether the parameter is variadic.
    pub variadic: bool,
    /// Default value of the parameter.
    pub default: Option<RString>,
}

/// Represents an exported class.
#[repr(C)]
pub struct Class {
    /// Name of the class.
    pub name: RString,
    /// Documentation comments for the class.
    pub docs: DocBlock,
    /// Name of the class the exported class extends. (Not implemented #326)
    pub extends: Option<RString>,
    /// Names of the interfaces the exported class implements. (Not implemented
    /// #326)
    pub implements: Vec<RString>,
    /// Properties of the class.
    pub properties: Vec<Property>,
    /// Methods of the class.
    pub methods: Vec<Method>,
    /// Constants of the class.
    pub constants: Vec<Constant>,
    /// Class flags
    pub flags: u32,
}

#[cfg(feature = "closure")]
impl Class {
    /// Creates a new class representing a Rust closure used for generating
    /// the stubs if the `closure` feature is enabled.
    #[must_use]
    pub fn closure() -> Self {
        Self {
            name: "RustClosure".into(),
            docs: DocBlock(StdVec::new().into()),
            extends: Option::None,
            implements: StdVec::new().into(),
            properties: StdVec::new().into(),
            methods: vec![Method {
                name: "__invoke".into(),
                docs: DocBlock(StdVec::new().into()),
                ty: MethodType::Member,
                params: vec![Parameter {
                    name: "args".into(),
                    ty: Option::Some(DataType::Mixed),
                    nullable: false,
                    variadic: true,
                    default: Option::None,
                }]
                .into(),
                retval: Option::Some(Retval {
                    ty: DataType::Mixed,
                    nullable: false,
                }),
                r#static: false,
                visibility: Visibility::Public,
                r#abstract: false,
            }]
            .into(),
            constants: StdVec::new().into(),
            flags: 0,
        }
    }
}

impl From<ClassBuilder> for Class {
    fn from(val: ClassBuilder) -> Self {
        let flags = val.get_flags();
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
                .map(Method::from)
                .collect::<StdVec<_>>()
                .into(),
            constants: val
                .constants
                .into_iter()
                .map(|(name, _, docs)| (name, docs))
                .map(Constant::from)
                .collect::<StdVec<_>>()
                .into(),
            flags,
        }
    }
}

#[cfg(feature = "enum")]
/// Represents an exported enum.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Enum {
    /// Name of the enum.
    pub name: RString,
    /// Documentation comments for the enum.
    pub docs: DocBlock,
    /// Cases of the enum.
    pub cases: Vec<EnumCase>,
    /// Backing type of the enum.
    pub backing_type: Option<RString>,
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
/// Represents a case in an exported enum.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct EnumCase {
    /// Name of the enum case.
    pub name: RString,
    /// Documentation comments for the enum case.
    pub docs: DocBlock,
    /// Value of the enum case.
    pub value: Option<RString>,
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

/// Represents a property attached to an exported class.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Property {
    /// Name of the property.
    pub name: RString,
    /// Documentation comments for the property.
    pub docs: DocBlock,
    /// Type of the property (Not implemented #376)
    pub ty: Option<DataType>,
    /// Visibility of the property.
    pub vis: Visibility,
    /// Whether the property is static.
    pub static_: bool,
    /// Whether the property is nullable. (Not implemented #376)
    pub nullable: bool,
    /// Default value of the property. (Not implemented #376)
    pub default: Option<RString>,
}

impl<D> From<(String, PropertyFlags, D, DocComments)> for Property {
    fn from(value: (String, PropertyFlags, D, DocComments)) -> Self {
        let (name, flags, _default, docs) = value;
        let static_ = flags.contains(PropertyFlags::Static);
        let vis = Visibility::from(flags);
        // TODO: Implement ty #376
        let ty = abi::Option::None;
        // TODO: Implement default #376
        let default = abi::Option::<abi::RString>::None;
        // TODO: Implement nullable #376
        let nullable = false;
        let docs = docs.into();

        Self {
            name: name.into(),
            docs,
            ty,
            vis,
            static_,
            nullable,
            default,
        }
    }
}

/// Represents a method attached to an exported class.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Method {
    /// Name of the method.
    pub name: RString,
    /// Documentation comments for the method.
    pub docs: DocBlock,
    /// Type of the method.
    pub ty: MethodType,
    /// Parameters of the method.
    pub params: Vec<Parameter>,
    /// Return value of the method.
    pub retval: Option<Retval>,
    /// Whether the method is static.
    pub r#static: bool,
    /// Visibility of the method.
    pub visibility: Visibility,
    /// Not describe method body, if is abstract.
    pub r#abstract: bool,
}

impl From<(FunctionBuilder<'_>, MethodFlags)> for Method {
    fn from(val: (FunctionBuilder<'_>, MethodFlags)) -> Self {
        let (builder, flags) = val;
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
}

/// Represents a value returned from a function or method.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Retval {
    /// Type of the return value.
    pub ty: DataType,
    /// Whether the return value is nullable.
    pub nullable: bool,
}

/// Enumerator used to differentiate between methods.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MethodType {
    /// A member method.
    Member,
    /// A static method.
    Static,
    /// A constructor.
    Constructor,
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

/// Enumerator used to differentiate between different method and property
/// visibilties.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Visibility {
    /// Private visibility.
    Private,
    /// Protected visibility.
    Protected,
    /// Public visibility.
    Public,
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

/// Represents an exported constant, stand alone or attached to a class.
#[repr(C)]
pub struct Constant {
    /// Name of the constant.
    pub name: RString,
    /// Documentation comments for the constant.
    pub docs: DocBlock,
    /// Value of the constant.
    pub value: Option<RString>,
}

impl From<(String, DocComments)> for Constant {
    fn from(val: (String, DocComments)) -> Self {
        let (name, docs) = val;
        Constant {
            name: name.into(),
            value: abi::Option::None,
            docs: docs.into(),
        }
    }
}

impl From<(String, Box<dyn IntoConst + Send>, DocComments)> for Constant {
    fn from(val: (String, Box<dyn IntoConst + Send + 'static>, DocComments)) -> Self {
        let (name, _, docs) = val;
        Constant {
            name: name.into(),
            value: abi::Option::None,
            docs: docs.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![cfg_attr(windows, feature(abi_vectorcall))]
    use cfg_if::cfg_if;

    use super::*;

    use crate::{args::Arg, test::test_function};

    #[test]
    fn test_new_description() {
        let module = Module {
            name: "test".into(),
            functions: vec![].into(),
            classes: vec![].into(),
            constants: vec![].into(),
            #[cfg(feature = "enum")]
            enums: vec![].into(),
        };

        let description = Description::new(module);
        assert_eq!(description.version, crate::VERSION);
        assert_eq!(description.module.name, "test".into());
    }

    #[test]
    fn test_doc_block_from() {
        let docs: &'static [&'static str] = &["doc1", "doc2"];
        let docs: DocBlock = docs.into();
        assert_eq!(docs.0.len(), 2);
        assert_eq!(docs.0[0], "doc1".into());
        assert_eq!(docs.0[1], "doc2".into());
    }

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
            .property("prop1", PropertyFlags::Public, None, &["doc1"])
            .method(
                FunctionBuilder::new("test_function", test_function),
                MethodFlags::Protected,
            );
        let class: Class = builder.into();

        assert_eq!(class.name, "TestClass".into());
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
    fn test_property_from() {
        let docs: &'static [&'static str] = &["doc1", "doc2"];
        let property: Property = (
            "test_property".to_string(),
            PropertyFlags::Protected,
            (),
            docs,
        )
            .into();
        assert_eq!(property.name, "test_property".into());
        assert_eq!(property.docs.0.len(), 2);
        assert_eq!(property.vis, Visibility::Protected);
        assert!(!property.static_);
        assert!(!property.nullable);
    }

    #[test]
    fn test_method_from() {
        let builder = FunctionBuilder::new("test_method", test_function)
            .docs(&["doc1", "doc2"])
            .arg(Arg::new("foo", DataType::Long))
            .returns(DataType::Bool, true, true);
        let method: Method = (builder, MethodFlags::Static | MethodFlags::Protected).into();
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
