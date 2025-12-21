use convert_case::{Case, Casing};
use darling::FromMeta;
use quote::{ToTokens, quote};
use syn::Ident;

/// Converts a Rust identifier to its PHP-compatible name.
///
/// This function strips the `r#` prefix from raw identifiers, since that prefix
/// is Rust-specific syntax for using reserved keywords as identifiers.
///
/// # Examples
///
/// ```ignore
/// use syn::parse_quote;
/// let ident: Ident = parse_quote!(r#as);
/// assert_eq!(ident_to_php_name(&ident), "as");
///
/// let ident: Ident = parse_quote!(normal_name);
/// assert_eq!(ident_to_php_name(&ident), "normal_name");
/// ```
pub fn ident_to_php_name(ident: &Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_string()
}

/// PHP reserved keywords that cannot be used as class, interface, trait, enum,
/// or function names.
///
/// See: <https://www.php.net/manual/en/reserved.keywords.php>
const PHP_RESERVED_KEYWORDS: &[&str] = &[
    // Keywords
    "__halt_compiler",
    "abstract",
    "and",
    "array",
    "as",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "declare",
    "default",
    "die",
    "do",
    "echo",
    "else",
    "elseif",
    "empty",
    "enum",
    "enddeclare",
    "endfor",
    "endforeach",
    "endif",
    "endswitch",
    "endwhile",
    "eval",
    "exit",
    "extends",
    "final",
    "finally",
    "fn",
    "for",
    "foreach",
    "function",
    "global",
    "goto",
    "if",
    "implements",
    "include",
    "include_once",
    "instanceof",
    "insteadof",
    "interface",
    "isset",
    "list",
    "match",
    "namespace",
    "new",
    "or",
    "print",
    "private",
    "protected",
    "public",
    "readonly",
    "require",
    "require_once",
    "return",
    "static",
    "switch",
    "throw",
    "trait",
    "try",
    "unset",
    "use",
    "var",
    "while",
    "xor",
    "yield",
    "yield from",
    // Compile-time constants
    "__CLASS__",
    "__DIR__",
    "__FILE__",
    "__FUNCTION__",
    "__LINE__",
    "__METHOD__",
    "__NAMESPACE__",
    "__TRAIT__",
    // Reserved classes (case-insensitive check needed)
    "self",
    "parent",
];

/// Type keywords that are reserved for class/interface/enum names but CAN be
/// used as method, function, constant, or property names in PHP.
///
/// Note: `resource` and `numeric` are NOT in this list because PHP allows them
/// as class names. See: <https://github.com/php/php-src/blob/master/Zend/zend_compile.c>
const PHP_TYPE_KEYWORDS: &[&str] = &[
    "bool", "false", "float", "int", "iterable", "mixed", "never", "null", "object", "string",
    "true", "void",
];

/// The context in which a PHP name is being used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhpNameContext {
    /// A class name (e.g., `class Foo {}`)
    Class,
    /// An interface name (e.g., `interface Foo {}`)
    Interface,
    /// An enum name (e.g., `enum Foo {}`)
    Enum,
    /// An enum case name (e.g., `case Foo;`)
    EnumCase,
    /// A function name (e.g., `function foo() {}`)
    Function,
    /// A method name (e.g., `public function foo() {}`)
    Method,
    /// A constant name (e.g., `const FOO = 1;`)
    Constant,
    /// A property name (e.g., `public $foo;`)
    Property,
}

impl PhpNameContext {
    fn description(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Enum => "enum",
            Self::EnumCase => "enum case",
            Self::Function => "function",
            Self::Method => "method",
            Self::Constant => "constant",
            Self::Property => "property",
        }
    }
}

/// Checks if a name is a PHP type keyword (case-insensitive).
///
/// Type keywords like `void`, `bool`, `int`, etc. are reserved for type declarations
/// but CAN be used as method, function, constant, or property names in PHP.
fn is_php_type_keyword(name: &str) -> bool {
    let lower = name.to_lowercase();
    PHP_TYPE_KEYWORDS
        .iter()
        .any(|&kw| kw.to_lowercase() == lower)
}

/// Checks if a name is a PHP reserved keyword (case-insensitive).
pub fn is_php_reserved_keyword(name: &str) -> bool {
    let lower = name.to_lowercase();
    PHP_RESERVED_KEYWORDS
        .iter()
        .any(|&kw| kw.to_lowercase() == lower)
}

/// Validates that a PHP name is not a reserved keyword.
///
/// The validation is context-aware:
/// - For class, interface, enum, and enum case names: both reserved keywords AND type keywords are checked
/// - For method, function, constant, and property names: only reserved keywords are checked
///   (type keywords like `void`, `bool`, etc. are allowed)
///
/// # Errors
///
/// Returns a `syn::Error` if the name is a reserved keyword in the given context.
pub fn validate_php_name(
    name: &str,
    context: PhpNameContext,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    let is_reserved = is_php_reserved_keyword(name);
    let is_type = is_php_type_keyword(name);

    // Type keywords are forbidden for class/interface/enum/enum case names
    let is_forbidden = match context {
        PhpNameContext::Class
        | PhpNameContext::Interface
        | PhpNameContext::Enum
        | PhpNameContext::EnumCase => is_reserved || is_type,
        PhpNameContext::Function
        | PhpNameContext::Method
        | PhpNameContext::Constant
        | PhpNameContext::Property => is_reserved,
    };

    if is_forbidden {
        return Err(syn::Error::new(
            span,
            format!(
                "cannot use '{}' as a PHP {} name: '{}' is a reserved keyword in PHP. \
                 Consider using a different name or the #[php(name = \"...\")] attribute to specify an alternative PHP name.",
                name,
                context.description(),
                name
            ),
        ));
    }

    Ok(())
}

const MAGIC_METHOD: [&str; 17] = [
    "__construct",
    "__destruct",
    "__call",
    "__call_static",
    "__get",
    "__set",
    "__isset",
    "__unset",
    "__sleep",
    "__wakeup",
    "__serialize",
    "__unserialize",
    "__to_string",
    "__invoke",
    "__set_state",
    "__clone",
    "__debug_info",
];

#[derive(Debug, FromMeta)]
pub enum Visibility {
    #[darling(rename = "public")]
    Public,
    #[darling(rename = "private")]
    Private,
    #[darling(rename = "protected")]
    Protected,
}

impl ToTokens for Visibility {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Visibility::Public => quote! { ::ext_php_rs::flags::MethodFlags::Public },
            Visibility::Protected => quote! { ::ext_php_rs::flags::MethodFlags::Protected },
            Visibility::Private => quote! { ::ext_php_rs::flags::MethodFlags::Private },
        }
        .to_tokens(tokens);
    }
}

pub trait Rename {
    fn rename(&self, rule: RenameRule) -> String;
}

pub trait MethodRename: Rename {
    fn rename_method(&self, rule: RenameRule) -> String;
}

#[derive(FromMeta, Debug, Default)]
#[darling(default)]
pub struct PhpRename {
    name: Option<String>,
    change_case: Option<RenameRule>,
}

impl PhpRename {
    pub fn rename(&self, name: impl AsRef<str>, default: RenameRule) -> String {
        if let Some(name) = self.name.as_ref() {
            name.clone()
        } else {
            name.as_ref().rename(self.change_case.unwrap_or(default))
        }
    }

    pub fn rename_method(&self, name: impl AsRef<str>, default: RenameRule) -> String {
        if let Some(name) = self.name.as_ref() {
            name.clone()
        } else {
            name.as_ref()
                .rename_method(self.change_case.unwrap_or(default))
        }
    }
}

#[derive(Debug, Copy, Clone, FromMeta, Default)]
pub enum RenameRule {
    /// Methods won't be renamed.
    #[darling(rename = "none")]
    None,
    /// Methods will be converted to `camelCase`.
    #[darling(rename = "camelCase")]
    #[default]
    Camel,
    /// Methods will be converted to `snake_case`.
    #[darling(rename = "snake_case")]
    Snake,
    /// Methods will be converted to `PascalCase`.
    #[darling(rename = "PascalCase")]
    Pascal,
    /// Renames to `UPPER_SNAKE_CASE`.
    #[darling(rename = "UPPER_CASE")]
    ScreamingSnake,
}

impl RenameRule {
    fn rename(self, value: impl AsRef<str>) -> String {
        match self {
            Self::None => value.as_ref().to_string(),
            Self::Camel => value.as_ref().to_case(Case::Camel),
            Self::Pascal => value.as_ref().to_case(Case::Pascal),
            Self::Snake => value.as_ref().to_case(Case::Snake),
            Self::ScreamingSnake => value.as_ref().to_case(Case::Constant),
        }
    }
}

impl<T> Rename for T
where
    T: ToString,
{
    fn rename(&self, rule: RenameRule) -> String {
        rule.rename(self.to_string())
    }
}

impl<T> MethodRename for T
where
    T: ToString + Rename,
{
    fn rename_method(&self, rule: RenameRule) -> String {
        let original = self.to_string();
        match rule {
            RenameRule::None => original,
            _ => {
                if MAGIC_METHOD.contains(&original.as_str()) {
                    match original.as_str() {
                        "__to_string" => "__toString".to_string(),
                        "__debug_info" => "__debugInfo".to_string(),
                        "__call_static" => "__callStatic".to_string(),
                        _ => original,
                    }
                } else {
                    self.rename(rule)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::{MethodRename, Rename};

    use super::{PhpRename, RenameRule};

    #[test]
    fn php_rename() {
        let rename = PhpRename {
            name: Some("test".to_string()),
            change_case: None,
        };
        assert_eq!(rename.rename("testCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename("TestCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename("TEST_CASE", RenameRule::Snake), "test");

        let rename = PhpRename {
            name: None,
            change_case: Some(RenameRule::ScreamingSnake),
        };
        assert_eq!(rename.rename("testCase", RenameRule::Snake), "TEST_CASE");
        assert_eq!(rename.rename("TestCase", RenameRule::Snake), "TEST_CASE");
        assert_eq!(rename.rename("TEST_CASE", RenameRule::Snake), "TEST_CASE");

        let rename = PhpRename {
            name: Some("test".to_string()),
            change_case: Some(RenameRule::ScreamingSnake),
        };
        assert_eq!(rename.rename("testCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename("TestCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename("TEST_CASE", RenameRule::Snake), "test");

        let rename = PhpRename {
            name: None,
            change_case: None,
        };
        assert_eq!(rename.rename("testCase", RenameRule::Snake), "test_case");
        assert_eq!(rename.rename("TestCase", RenameRule::Snake), "test_case");
        assert_eq!(rename.rename("TEST_CASE", RenameRule::Snake), "test_case");
    }

    #[test]
    fn php_rename_method() {
        let rename = PhpRename {
            name: Some("test".to_string()),
            change_case: None,
        };
        assert_eq!(rename.rename_method("testCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename_method("TestCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename_method("TEST_CASE", RenameRule::Snake), "test");

        let rename = PhpRename {
            name: None,
            change_case: Some(RenameRule::ScreamingSnake),
        };
        assert_eq!(
            rename.rename_method("testCase", RenameRule::Snake),
            "TEST_CASE"
        );
        assert_eq!(
            rename.rename_method("TestCase", RenameRule::Snake),
            "TEST_CASE"
        );
        assert_eq!(
            rename.rename_method("TEST_CASE", RenameRule::Snake),
            "TEST_CASE"
        );

        let rename = PhpRename {
            name: Some("test".to_string()),
            change_case: Some(RenameRule::ScreamingSnake),
        };
        assert_eq!(rename.rename_method("testCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename_method("TestCase", RenameRule::Snake), "test");
        assert_eq!(rename.rename_method("TEST_CASE", RenameRule::Snake), "test");

        let rename = PhpRename {
            name: None,
            change_case: None,
        };
        assert_eq!(
            rename.rename_method("testCase", RenameRule::Snake),
            "test_case"
        );
        assert_eq!(
            rename.rename_method("TestCase", RenameRule::Snake),
            "test_case"
        );
        assert_eq!(
            rename.rename_method("TEST_CASE", RenameRule::Snake),
            "test_case"
        );
    }

    #[test]
    fn rename_magic_method() {
        for &(magic, expected) in &[
            ("__construct", "__construct"),
            ("__destruct", "__destruct"),
            ("__call", "__call"),
            ("__call_static", "__callStatic"),
            ("__get", "__get"),
            ("__set", "__set"),
            ("__isset", "__isset"),
            ("__unset", "__unset"),
            ("__sleep", "__sleep"),
            ("__wakeup", "__wakeup"),
            ("__serialize", "__serialize"),
            ("__unserialize", "__unserialize"),
            ("__to_string", "__toString"),
            ("__invoke", "__invoke"),
            ("__set_state", "__set_state"),
            ("__clone", "__clone"),
            ("__debug_info", "__debugInfo"),
        ] {
            assert_eq!(magic, magic.rename_method(RenameRule::None));
            assert_eq!(
                magic,
                PhpRename {
                    name: None,
                    change_case: Some(RenameRule::None)
                }
                .rename_method(magic, RenameRule::ScreamingSnake)
            );

            assert_eq!(expected, magic.rename_method(RenameRule::Camel));
            assert_eq!(
                expected,
                PhpRename {
                    name: None,
                    change_case: Some(RenameRule::Camel)
                }
                .rename_method(magic, RenameRule::ScreamingSnake)
            );

            assert_eq!(expected, magic.rename_method(RenameRule::Pascal));
            assert_eq!(
                expected,
                PhpRename {
                    name: None,
                    change_case: Some(RenameRule::Pascal)
                }
                .rename_method(magic, RenameRule::ScreamingSnake)
            );

            assert_eq!(expected, magic.rename_method(RenameRule::Snake));
            assert_eq!(
                expected,
                PhpRename {
                    name: None,
                    change_case: Some(RenameRule::Snake)
                }
                .rename_method(magic, RenameRule::ScreamingSnake)
            );

            assert_eq!(expected, magic.rename_method(RenameRule::ScreamingSnake));
            assert_eq!(
                expected,
                PhpRename {
                    name: None,
                    change_case: Some(RenameRule::ScreamingSnake)
                }
                .rename_method(magic, RenameRule::Camel)
            );
        }
    }

    #[test]
    fn rename_method() {
        let &(original, camel, snake, pascal, screaming_snake) =
            &("get_name", "getName", "get_name", "GetName", "GET_NAME");
        assert_eq!(original, original.rename_method(RenameRule::None));
        assert_eq!(camel, original.rename_method(RenameRule::Camel));
        assert_eq!(pascal, original.rename_method(RenameRule::Pascal));
        assert_eq!(snake, original.rename_method(RenameRule::Snake));
        assert_eq!(
            screaming_snake,
            original.rename_method(RenameRule::ScreamingSnake)
        );
    }

    #[test]
    fn rename() {
        let &(original, camel, snake, pascal, screaming_snake) =
            &("get_name", "getName", "get_name", "GetName", "GET_NAME");
        assert_eq!(original, original.rename(RenameRule::None));
        assert_eq!(camel, original.rename(RenameRule::Camel));
        assert_eq!(pascal, original.rename(RenameRule::Pascal));
        assert_eq!(snake, original.rename(RenameRule::Snake));
        assert_eq!(screaming_snake, original.rename(RenameRule::ScreamingSnake));
    }

    #[test]
    fn ident_to_php_name_strips_raw_prefix() {
        use super::ident_to_php_name;
        use syn::parse_quote;

        // Raw identifier should have r# prefix stripped
        let raw_ident: syn::Ident = parse_quote!(r#as);
        assert_eq!(ident_to_php_name(&raw_ident), "as");

        let raw_ident: syn::Ident = parse_quote!(r#match);
        assert_eq!(ident_to_php_name(&raw_ident), "match");

        let raw_ident: syn::Ident = parse_quote!(r#type);
        assert_eq!(ident_to_php_name(&raw_ident), "type");

        // Normal identifiers should be unchanged
        let normal_ident: syn::Ident = parse_quote!(normal_name);
        assert_eq!(ident_to_php_name(&normal_ident), "normal_name");

        let normal_ident: syn::Ident = parse_quote!(foo);
        assert_eq!(ident_to_php_name(&normal_ident), "foo");
    }

    #[test]
    fn test_is_php_reserved_keyword() {
        use super::is_php_reserved_keyword;

        // Hard keywords should be detected
        assert!(is_php_reserved_keyword("class"));
        assert!(is_php_reserved_keyword("function"));
        assert!(is_php_reserved_keyword("match"));

        // Case-insensitive
        assert!(is_php_reserved_keyword("CLASS"));
        assert!(is_php_reserved_keyword("FUNCTION"));

        // Type keywords are NOT in the reserved list (they're in PHP_TYPE_KEYWORDS)
        assert!(!is_php_reserved_keyword("void"));
        assert!(!is_php_reserved_keyword("true"));
        assert!(!is_php_reserved_keyword("bool"));

        // Non-keywords should pass
        assert!(!is_php_reserved_keyword("MyClass"));
        assert!(!is_php_reserved_keyword("foo"));
    }

    #[test]
    fn test_validate_php_name_rejects_reserved_keyword() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        let result = validate_php_name("class", PhpNameContext::Class, Span::call_site());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("is a reserved keyword in PHP"));
    }

    #[test]
    fn test_validate_php_name_rejects_type_keyword_for_class() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // Type keywords like 'void' cannot be used as class names
        let result = validate_php_name("void", PhpNameContext::Class, Span::call_site());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("is a reserved keyword in PHP"));
    }

    #[test]
    fn test_validate_php_name_rejects_type_keyword_for_enum_case() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // Type keywords like 'true' cannot be used as enum case names
        let result = validate_php_name("true", PhpNameContext::EnumCase, Span::call_site());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("is a reserved keyword in PHP"));

        let result = validate_php_name("false", PhpNameContext::EnumCase, Span::call_site());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_php_name_allows_type_keyword_for_method() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // Type keywords like 'void' CAN be used as method names in PHP
        validate_php_name("void", PhpNameContext::Method, Span::call_site()).unwrap();
        validate_php_name("true", PhpNameContext::Method, Span::call_site()).unwrap();
        validate_php_name("bool", PhpNameContext::Method, Span::call_site()).unwrap();
        validate_php_name("int", PhpNameContext::Method, Span::call_site()).unwrap();
    }

    #[test]
    fn test_validate_php_name_allows_type_keyword_for_function() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // Type keywords CAN be used as function names in PHP
        validate_php_name("void", PhpNameContext::Function, Span::call_site()).unwrap();
    }

    #[test]
    fn test_validate_php_name_allows_type_keyword_for_constant() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // Type keywords CAN be used as constant names in PHP
        validate_php_name("void", PhpNameContext::Constant, Span::call_site()).unwrap();
    }

    #[test]
    fn test_validate_php_name_allows_resource_and_numeric_for_class() {
        use super::{PhpNameContext, validate_php_name};
        use proc_macro2::Span;

        // 'resource' and 'numeric' are NOT reserved for class names in PHP
        validate_php_name("resource", PhpNameContext::Class, Span::call_site()).unwrap();
        validate_php_name("numeric", PhpNameContext::Class, Span::call_site()).unwrap();
    }
}
