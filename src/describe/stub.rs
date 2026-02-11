//! Traits and implementations to convert describe units into PHP stub code.

use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Error as FmtError, Result as FmtResult, Write},
    option::Option as StdOption,
    vec::Vec as StdVec,
};

use super::{
    Class, Constant, DocBlock, Function, Method, MethodType, Module, Parameter, Property, Retval,
    Visibility,
    abi::{Option, RString, Str},
};

#[cfg(feature = "enum")]
use crate::describe::{Enum, EnumCase};
use crate::flags::{ClassFlags, DataType};

/// Parsed rustdoc sections for conversion to `PHPDoc`.
#[derive(Default)]
struct ParsedRustDoc {
    /// Summary/description lines (before any section header).
    summary: StdVec<String>,
    /// Parameter descriptions from `# Arguments` section.
    /// Maps parameter name to description.
    params: HashMap<String, String>,
    /// Parameter type overrides from `# Parameters` section.
    /// Maps parameter name to PHP type string (e.g., "?string &$stdout").
    /// Used to override `mixed` type in stubs when Rust uses `Zval`.
    param_types: HashMap<String, String>,
    /// Return value description from `# Returns` section.
    returns: StdOption<String>,
    /// Error descriptions from `# Errors` section (for @throws).
    errors: StdVec<String>,
}

/// Parse rustdoc-style documentation into structured sections.
fn parse_rustdoc(docs: &[Str]) -> ParsedRustDoc {
    let mut result = ParsedRustDoc::default();
    let mut current_section: StdOption<&str> = None;
    let mut section_content: StdVec<String> = StdVec::new();

    for line in docs {
        let line = line.as_ref();
        let trimmed = line.trim();

        // Check for section headers (# Arguments, # Returns, # Errors, etc.)
        if trimmed.starts_with("# ") {
            // Save previous section content
            finalize_section(&mut result, current_section, &section_content);
            section_content.clear();

            // Start new section
            let section_name = trimmed.strip_prefix("# ").unwrap_or(trimmed);
            current_section = Some(section_name);
        } else if current_section.is_some() {
            // Inside a section, collect content
            section_content.push(line.to_string());
        } else {
            // Before any section header - this is the summary
            result.summary.push(line.to_string());
        }
    }

    // Finalize last section
    finalize_section(&mut result, current_section, &section_content);

    result
}

/// Process section content and store in the appropriate field.
fn finalize_section(result: &mut ParsedRustDoc, section: StdOption<&str>, content: &[String]) {
    let Some(section_name) = section else {
        return;
    };

    match section_name {
        "Arguments" => {
            // Parse argument list: `* `name` - description` or `* name - description`
            for line in content {
                let trimmed = line.trim();
                let item = trimmed
                    .strip_prefix("* ")
                    .or_else(|| trimmed.strip_prefix("- "));
                // Try to extract parameter name and description
                // Format: `name` - description OR name - description
                if let Some(item) = item
                    && let Some((name, desc)) = parse_param_line(item.trim())
                {
                    result.params.insert(name, desc);
                }
            }
        }
        "Returns" => {
            // Collect all non-empty lines as return description
            let desc: String = content
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<StdVec<_>>()
                .join(" ");
            if !desc.is_empty() {
                result.returns = Some(desc);
            }
        }
        "Errors" => {
            // Collect error descriptions for @throws
            for line in content {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    result.errors.push(trimmed.to_string());
                }
            }
        }
        "Parameters" => {
            // Parse parameter list with type overrides
            // Format: - `name`: `type` description
            for line in content {
                let trimmed = line.trim();
                let item = trimmed
                    .strip_prefix("* ")
                    .or_else(|| trimmed.strip_prefix("- "));
                if let Some(item) = item
                    && let Some((name, ty, desc)) = parse_typed_param_line(item.trim())
                {
                    result.param_types.insert(name.clone(), ty);
                    if !desc.is_empty() {
                        result.params.insert(name, desc);
                    }
                }
            }
        }
        // Ignore other sections like Examples, Panics, Safety, etc.
        _ => {}
    }
}

/// Parse a parameter line from rustdoc `# Arguments` section.
/// Handles formats like:
/// - `name` - description
/// - name - description
/// - `$name` - description
fn parse_param_line(line: &str) -> StdOption<(String, String)> {
    // Try backtick format first: `name` - description
    if let Some(rest) = line.strip_prefix('`')
        && let Some(end_tick) = rest.find('`')
    {
        let name = &rest[..end_tick];
        // Skip the closing backtick and find the separator
        let after_tick = &rest[end_tick + 1..];
        let desc = after_tick
            .trim()
            .strip_prefix('-')
            .or_else(|| after_tick.trim().strip_prefix(':'))
            .map_or_else(|| after_tick.trim(), str::trim);
        // Remove leading $ if present
        let name = name.strip_prefix('$').unwrap_or(name);
        return Some((name.to_string(), desc.to_string()));
    }

    // Try simple format: name - description
    if let Some(sep_pos) = line.find(" - ") {
        let name = line[..sep_pos].trim();
        let desc = line[sep_pos + 3..].trim();
        // Remove leading $ if present
        let name = name.strip_prefix('$').unwrap_or(name);
        return Some((name.to_string(), desc.to_string()));
    }

    None
}

/// Parse a typed parameter line from rustdoc `# Parameters` section.
/// Format: `name`: `type` description
/// Returns (name, type, description) if successful.
fn parse_typed_param_line(line: &str) -> StdOption<(String, String, String)> {
    // Expected format: `name`: `type` description
    // First, extract the name in backticks
    let rest = line.strip_prefix('`')?;
    let end_tick = rest.find('`')?;
    let name = rest[..end_tick].to_string();

    // Skip to after the colon
    let after_name = rest[end_tick + 1..].trim();
    let after_colon = after_name.strip_prefix(':')?.trim();

    // Extract the type in backticks
    let type_rest = after_colon.strip_prefix('`')?;
    let type_end_tick = type_rest.find('`')?;
    let ty = type_rest[..type_end_tick].to_string();

    // The rest is the description
    let desc = type_rest[type_end_tick + 1..].trim().to_string();

    // Clean up the name (remove $ prefix if present)
    let name = name.strip_prefix('$').unwrap_or(&name).to_string();

    Some((name, ty, desc))
}

/// Format a `PHPDoc` comment block for a function or method.
///
/// Converts rustdoc-style documentation to `PHPDoc` format, including:
/// - Summary/description
/// - @param tags from `# Arguments` section
/// - @return tag from `# Returns` section
/// - @throws tags from `# Errors` section
///
/// Returns the parameter type overrides map for use in stub signature generation.
fn format_phpdoc(
    docs: &DocBlock,
    params: &[Parameter],
    ret: StdOption<&Retval>,
    buf: &mut String,
) -> Result<HashMap<String, String>, FmtError> {
    if docs.0.is_empty() && params.is_empty() && ret.is_none() {
        return Ok(HashMap::new());
    }

    let parsed = parse_rustdoc(&docs.0);

    // Check if we have any content to output
    let has_summary = parsed.summary.iter().any(|s| !s.trim().is_empty());
    let has_params = !params.is_empty();
    let has_return = ret.is_some();
    let has_errors = !parsed.errors.is_empty();

    if !has_summary && !has_params && !has_return && !has_errors {
        return Ok(parsed.param_types);
    }

    writeln!(buf, "/**")?;

    // Output summary (trim trailing empty lines)
    let summary_lines: StdVec<_> = parsed
        .summary
        .iter()
        .rev()
        .skip_while(|s| s.trim().is_empty())
        .collect::<StdVec<_>>()
        .into_iter()
        .rev()
        .collect();

    for line in &summary_lines {
        writeln!(buf, " *{line}")?;
    }

    // Add blank line before tags if we have summary and tags
    if !summary_lines.is_empty() && (has_params || has_return || has_errors) {
        writeln!(buf, " *")?;
    }

    // Output @param tags
    for param in params {
        // Use type override from # Parameters section if available and type is mixed
        let type_str = if let Some(type_override) = parsed.param_types.get(param.name.as_ref()) {
            // Extract just the type part (strip reference like &$name)
            extract_php_type(type_override)
        } else {
            match &param.ty {
                Option::Some(ty) => datatype_to_phpdoc(ty, param.nullable),
                Option::None => "mixed".to_string(),
            }
        };

        let desc = parsed.params.get(param.name.as_ref()).cloned();
        if let Some(desc) = desc {
            writeln!(buf, " * @param {type_str} ${} {desc}", param.name)?;
        } else {
            writeln!(buf, " * @param {type_str} ${}", param.name)?;
        }
    }

    // Output @return tag
    if let Some(retval) = ret {
        let type_str = datatype_to_phpdoc(&retval.ty, retval.nullable);
        if let Some(desc) = &parsed.returns {
            writeln!(buf, " * @return {type_str} {desc}")?;
        } else {
            writeln!(buf, " * @return {type_str}")?;
        }
    }

    // Output @throws tags
    for error in &parsed.errors {
        writeln!(buf, " * @throws \\Exception {error}")?;
    }

    writeln!(buf, " */")?;
    Ok(parsed.param_types)
}

/// Extract the PHP type from a type override string.
/// Handles formats like "?string &$name" -> "?string"
fn extract_php_type(type_str: &str) -> String {
    // The type override might contain reference notation like "&$name"
    // We want just the type part
    type_str
        .split_whitespace()
        .next()
        .unwrap_or("mixed")
        .to_string()
}

/// Convert a `DataType` to `PHPDoc` type string.
fn datatype_to_phpdoc(ty: &DataType, nullable: bool) -> String {
    let base = match ty {
        DataType::Bool | DataType::True | DataType::False => "bool",
        DataType::Long => "int",
        DataType::Double => "float",
        DataType::String => "string",
        DataType::Array => "array",
        DataType::Object(Some(name)) => return format_class_type(name, nullable),
        DataType::Object(None) => "object",
        DataType::Resource => "resource",
        DataType::Callable => "callable",
        DataType::Void => "void",
        DataType::Null => "null",
        DataType::Iterable => "iterable",
        // Mixed, Undef, Ptr, Indirect, Reference, ConstantExpression
        _ => "mixed",
    };

    if nullable && !matches!(ty, DataType::Mixed | DataType::Null | DataType::Void) {
        format!("{base}|null")
    } else {
        base.to_string()
    }
}

/// Format a class type for `PHPDoc` (with backslash prefix).
fn format_class_type(name: &str, nullable: bool) -> String {
    let class_name = if name.starts_with('\\') {
        name.to_string()
    } else {
        format!("\\{name}")
    };

    if nullable {
        format!("{class_name}|null")
    } else {
        class_name
    }
}

/// Implemented on types which can be converted into PHP stubs.
pub trait ToStub {
    /// Converts the implementor into PHP code, represented as a PHP stub.
    /// Returned as a string.
    ///
    /// # Returns
    ///
    /// Returns a string on success.
    ///
    /// # Errors
    ///
    /// Returns an error if there was an error writing into the string.
    fn to_stub(&self) -> Result<String, FmtError> {
        let mut buf = String::new();
        self.fmt_stub(&mut buf)?;
        Ok(buf)
    }

    /// Converts the implementor into PHP code, represented as a PHP stub.
    ///
    /// # Parameters
    ///
    /// * `buf` - The buffer to write the PHP code into.
    ///
    /// # Returns
    ///
    /// Returns nothing on success.
    ///
    /// # Errors
    ///
    /// Returns an error if there was an error writing into the buffer.
    fn fmt_stub(&self, buf: &mut String) -> FmtResult;
}

impl ToStub for Module {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        writeln!(buf, "<?php")?;
        writeln!(buf)?;
        writeln!(buf, "// Stubs for {}", self.name.as_ref())?;
        writeln!(buf)?;

        // To account for namespaces we need to group by them. [`None`] as the key
        // represents no namespace, while [`Some`] represents a namespace.
        // Store (sort_key, stub) tuples to sort by name, not by rendered output.
        let mut entries: HashMap<StdOption<&str>, StdVec<(String, String)>> = HashMap::new();

        // Inserts a value into the entries hashmap. Takes a key, sort key, and entry,
        // creating the internal vector if it doesn't already exist.
        let mut insert = |ns, sort_key: String, entry| {
            let bucket = entries.entry(ns).or_default();
            bucket.push((sort_key, entry));
        };

        for c in &*self.constants {
            let (ns, name) = split_namespace(c.name.as_ref());
            insert(ns, name.to_string(), c.to_stub()?);
        }

        for func in &*self.functions {
            let (ns, name) = split_namespace(func.name.as_ref());
            insert(ns, name.to_string(), func.to_stub()?);
        }

        for class in &*self.classes {
            let (ns, name) = split_namespace(class.name.as_ref());
            insert(ns, name.to_string(), class.to_stub()?);
        }

        #[cfg(feature = "enum")]
        for r#enum in &*self.enums {
            let (ns, name) = split_namespace(r#enum.name.as_ref());
            insert(ns, name.to_string(), r#enum.to_stub()?);
        }

        // Sort by entity name, not by rendered output
        for bucket in entries.values_mut() {
            bucket.sort_by(|(a, _), (b, _)| a.cmp(b));
        }

        let mut entries: StdVec<_> = entries.iter().collect();
        entries.sort_by(|(l, _), (r, _)| match (l, r) {
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(l), Some(r)) => l.cmp(r),
        });

        buf.push_str(
            &entries
                .into_iter()
                .map(|(ns, entries)| {
                    let mut buf = String::new();
                    if let Some(ns) = ns {
                        writeln!(buf, "namespace {ns} {{")?;
                    } else {
                        writeln!(buf, "namespace {{")?;
                    }

                    buf.push_str(
                        &entries
                            .iter()
                            .map(|(_, stub)| indent(stub, 4))
                            .collect::<StdVec<_>>()
                            .join(NEW_LINE_SEPARATOR),
                    );

                    writeln!(buf, "}}")?;
                    Ok(buf)
                })
                .collect::<Result<StdVec<_>, FmtError>>()?
                .join(NEW_LINE_SEPARATOR),
        );

        Ok(())
    }
}

impl ToStub for Function {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        // Convert rustdoc to PHPDoc format (Issue #369)
        let ret_ref = match &self.ret {
            Option::Some(r) => Some(r),
            Option::None => None,
        };
        let type_overrides = format_phpdoc(&self.docs, &self.params, ret_ref, buf)?;

        let (_, name) = split_namespace(self.name.as_ref());

        // Render parameters with type overrides
        let params_str = self
            .params
            .iter()
            .map(|p| param_to_stub(p, &type_overrides))
            .collect::<Result<StdVec<_>, FmtError>>()?
            .join(", ");

        write!(buf, "function {name}({params_str})")?;

        if let Option::Some(retval) = &self.ret {
            write!(buf, ": ")?;
            // Don't add ? for mixed/null/void - they already include null or can't be nullable
            if retval.nullable
                && !matches!(retval.ty, DataType::Mixed | DataType::Null | DataType::Void)
            {
                write!(buf, "?")?;
            }
            retval.ty.fmt_stub(buf)?;
        }

        writeln!(buf, " {{}}")
    }
}

/// Render a parameter to stub format, with optional type overrides from rustdoc.
///
/// When a parameter's Rust type is `Zval` (which maps to `mixed` in PHP), the
/// `# Parameters` section in rustdoc can specify a more precise PHP type.
fn param_to_stub(
    param: &Parameter,
    type_overrides: &HashMap<String, String>,
) -> Result<String, FmtError> {
    let mut buf = String::new();

    // Check if we should use a type override from # Parameters section
    // Only use override if the param type is Mixed (i.e., Zval in Rust)
    let type_override = type_overrides
        .get(param.name.as_ref())
        .filter(|_| matches!(&param.ty, Option::Some(DataType::Mixed) | Option::None));

    if let Some(override_str) = type_override {
        // Use the documented type from # Parameters
        let type_str = extract_php_type(override_str);
        write!(buf, "{type_str} ")?;
    } else if let Option::Some(ty) = &param.ty {
        // Don't add ? for mixed/null/void - they already include null or can't be nullable
        if param.nullable && !matches!(ty, DataType::Mixed | DataType::Null | DataType::Void) {
            write!(buf, "?")?;
        }
        ty.fmt_stub(&mut buf)?;
        write!(buf, " ")?;
    }

    if param.variadic {
        write!(buf, "...")?;
    }

    write!(buf, "${}", param.name)?;

    // Add default value to stub
    if let Option::Some(default) = &param.default {
        write!(buf, " = {default}")?;
    } else if param.nullable {
        // For nullable parameters without explicit default, add = null
        // This makes Option<T> parameters truly optional in PHP
        write!(buf, " = null")?;
    }

    Ok(buf)
}

impl ToStub for Parameter {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        let empty_overrides = HashMap::new();
        let result = param_to_stub(self, &empty_overrides)?;
        buf.push_str(&result);
        Ok(())
    }
}

impl ToStub for DataType {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        let mut fqdn = "\\".to_owned();
        write!(
            buf,
            "{}",
            match self {
                DataType::Bool | DataType::True | DataType::False => "bool",
                DataType::Long => "int",
                DataType::Double => "float",
                DataType::String => "string",
                DataType::Array => "array",
                DataType::Object(Some(ty)) => {
                    fqdn.push_str(ty);
                    fqdn.as_str()
                }
                DataType::Object(None) => "object",
                DataType::Resource => "resource",
                DataType::Reference => "reference",
                DataType::Callable => "callable",
                DataType::Iterable => "iterable",
                DataType::Void => "void",
                DataType::Null => "null",
                DataType::Mixed
                | DataType::Undef
                | DataType::Ptr
                | DataType::Indirect
                | DataType::ConstantExpression => "mixed",
            }
        )
    }
}

impl ToStub for DocBlock {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        if !self.0.is_empty() {
            writeln!(buf, "/**")?;
            for comment in self.0.iter() {
                writeln!(buf, " *{comment}")?;
            }
            writeln!(buf, " */")?;
        }
        Ok(())
    }
}

impl ToStub for Class {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        self.docs.fmt_stub(buf)?;

        let (_, name) = split_namespace(self.name.as_ref());
        let flags = ClassFlags::from_bits(self.flags).unwrap_or(ClassFlags::empty());
        let is_interface = flags.contains(ClassFlags::Interface);

        if is_interface {
            write!(buf, "interface {name} ")?;
        } else {
            write!(buf, "class {name} ")?;
        }

        if let Option::Some(extends) = &self.extends {
            write!(buf, "extends {extends} ")?;
        }

        if !self.implements.is_empty() && !is_interface {
            write!(
                buf,
                "implements {} ",
                self.implements
                    .iter()
                    .map(RString::as_str)
                    .collect::<StdVec<_>>()
                    .join(", ")
            )?;
        }

        if !self.implements.is_empty() && is_interface {
            write!(
                buf,
                "extends {} ",
                self.implements
                    .iter()
                    .map(RString::as_str)
                    .collect::<StdVec<_>>()
                    .join(", ")
            )?;
        }

        writeln!(buf, "{{")?;

        // Collect (sort_key, stub) tuples to sort by name, not by rendered output
        let mut constants: StdVec<_> = self
            .constants
            .iter()
            .map(|c| {
                c.to_stub()
                    .map(|s| (c.name.as_ref().to_string(), indent(&s, 4)))
            })
            .collect::<Result<_, FmtError>>()?;
        let mut properties: StdVec<_> = self
            .properties
            .iter()
            .map(|p| {
                p.to_stub()
                    .map(|s| (p.name.as_ref().to_string(), indent(&s, 4)))
            })
            .collect::<Result<_, FmtError>>()?;
        let mut methods: StdVec<_> = self
            .methods
            .iter()
            .map(|m| {
                m.to_stub()
                    .map(|s| (m.name.as_ref().to_string(), indent(&s, 4)))
            })
            .collect::<Result<_, FmtError>>()?;

        // Sort by entity name
        constants.sort_by(|(a, _), (b, _)| a.cmp(b));
        properties.sort_by(|(a, _), (b, _)| a.cmp(b));
        methods.sort_by(|(a, _), (b, _)| a.cmp(b));

        buf.push_str(
            &constants
                .into_iter()
                .chain(properties)
                .chain(methods)
                .map(|(_, stub)| stub)
                .collect::<StdVec<_>>()
                .join(NEW_LINE_SEPARATOR),
        );

        writeln!(buf, "}}")
    }
}

#[cfg(feature = "enum")]
impl ToStub for Enum {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        self.docs.fmt_stub(buf)?;

        let (_, name) = split_namespace(self.name.as_ref());
        write!(buf, "enum {name}")?;

        if let Option::Some(backing_type) = &self.backing_type {
            write!(buf, ": {backing_type}")?;
        }

        writeln!(buf, " {{")?;

        for case in self.cases.iter() {
            case.fmt_stub(buf)?;
        }

        writeln!(buf, "}}")
    }
}

#[cfg(feature = "enum")]
impl ToStub for EnumCase {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        self.docs.fmt_stub(buf)?;

        write!(buf, "  case {}", self.name)?;
        if let Option::Some(value) = &self.value {
            write!(buf, " = {value}")?;
        }
        writeln!(buf, ";")
    }
}

impl ToStub for Property {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        self.docs.fmt_stub(buf)?;
        self.vis.fmt_stub(buf)?;

        write!(buf, " ")?;

        if self.static_ {
            write!(buf, "static ")?;
        }
        if let Option::Some(ty) = &self.ty {
            ty.fmt_stub(buf)?;
        }
        write!(buf, "${}", self.name)?;
        if let Option::Some(default) = &self.default {
            write!(buf, " = {default}")?;
        }
        writeln!(buf, ";")
    }
}

impl ToStub for Visibility {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        write!(
            buf,
            "{}",
            match self {
                Visibility::Private => "private",
                Visibility::Protected => "protected",
                Visibility::Public => "public",
            }
        )
    }
}

impl ToStub for Method {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        // Convert rustdoc to PHPDoc format (Issue #369)
        // Don't include return type for constructors in PHPDoc
        let ret_ref = if matches!(self.ty, MethodType::Constructor) {
            None
        } else {
            match &self.retval {
                Option::Some(r) => Some(r),
                Option::None => None,
            }
        };
        let type_overrides = format_phpdoc(&self.docs, &self.params, ret_ref, buf)?;

        self.visibility.fmt_stub(buf)?;

        write!(buf, " ")?;

        if matches!(self.ty, MethodType::Static) {
            write!(buf, "static ")?;
        }

        // Render parameters with type overrides
        let params_str = self
            .params
            .iter()
            .map(|p| param_to_stub(p, &type_overrides))
            .collect::<Result<StdVec<_>, FmtError>>()?
            .join(", ");

        write!(buf, "function {}({params_str})", self.name)?;

        if !matches!(self.ty, MethodType::Constructor)
            && let Option::Some(retval) = &self.retval
        {
            write!(buf, ": ")?;
            // Don't add ? for mixed/null/void - they already include null or can't be nullable
            if retval.nullable
                && !matches!(retval.ty, DataType::Mixed | DataType::Null | DataType::Void)
            {
                write!(buf, "?")?;
            }
            retval.ty.fmt_stub(buf)?;
        }

        if self.r#abstract {
            writeln!(buf, ";")
        } else {
            writeln!(buf, " {{}}")
        }
    }
}

impl ToStub for Constant {
    fn fmt_stub(&self, buf: &mut String) -> FmtResult {
        self.docs.fmt_stub(buf)?;

        write!(buf, "const {} = ", self.name)?;
        if let Option::Some(value) = &self.value {
            write!(buf, "{value}")?;
        } else {
            write!(buf, "null")?;
        }
        writeln!(buf, ";")
    }
}

#[cfg(windows)]
const NEW_LINE_SEPARATOR: &str = "\r\n";
#[cfg(not(windows))]
const NEW_LINE_SEPARATOR: &str = "\n";

/// Takes a class name and splits the namespace off from the actual class name.
///
/// # Returns
///
/// A tuple, where the first item is the namespace (or [`None`] if not
/// namespaced), and the second item is the class name.
fn split_namespace(class: &str) -> (StdOption<&str>, &str) {
    let idx = class.rfind('\\');

    if let Some(idx) = idx {
        (Some(&class[0..idx]), &class[idx + 1..])
    } else {
        (None, class)
    }
}

/// Indents a given string to a given depth. Depth is given in number of spaces
/// to be appended. Returns a new string with the new indentation. Will not
/// indent whitespace lines.
///
/// # Parameters
///
/// * `s` - The string to indent.
/// * `depth` - The depth to indent the lines to, in spaces.
///
/// # Returns
///
/// The indented string.
fn indent(s: &str, depth: usize) -> String {
    let indent = format!("{:depth$}", "", depth = depth);

    s.split('\n')
        .map(|line| {
            let mut result = String::new();
            if line.chars().any(|c| !c.is_whitespace()) {
                result.push_str(&indent);
                result.push_str(line);
            }
            result
        })
        .collect::<StdVec<_>>()
        .join(NEW_LINE_SEPARATOR)
}

#[cfg(test)]
mod test {
    use super::{ToStub, split_namespace};
    use crate::flags::DataType;

    #[test]
    pub fn test_split_ns() {
        assert_eq!(split_namespace("ext\\php\\rs"), (Some("ext\\php"), "rs"));
        assert_eq!(split_namespace("test_solo_ns"), (None, "test_solo_ns"));
        assert_eq!(split_namespace("simple\\ns"), (Some("simple"), "ns"));
    }

    #[test]
    #[cfg(not(windows))]
    #[allow(clippy::uninlined_format_args)]
    pub fn test_indent() {
        use super::indent;
        use crate::describe::stub::NEW_LINE_SEPARATOR;

        assert_eq!(indent("hello", 4), "    hello");
        assert_eq!(
            indent(&format!("hello{nl}world{nl}", nl = NEW_LINE_SEPARATOR), 4),
            format!("    hello{nl}    world{nl}", nl = NEW_LINE_SEPARATOR)
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    pub fn test_datatype_to_stub() {
        // Test that all DataType variants produce correct PHP type strings
        assert_eq!(DataType::Void.to_stub().unwrap(), "void");
        assert_eq!(DataType::Null.to_stub().unwrap(), "null");
        assert_eq!(DataType::Bool.to_stub().unwrap(), "bool");
        assert_eq!(DataType::True.to_stub().unwrap(), "bool");
        assert_eq!(DataType::False.to_stub().unwrap(), "bool");
        assert_eq!(DataType::Long.to_stub().unwrap(), "int");
        assert_eq!(DataType::Double.to_stub().unwrap(), "float");
        assert_eq!(DataType::String.to_stub().unwrap(), "string");
        assert_eq!(DataType::Array.to_stub().unwrap(), "array");
        assert_eq!(DataType::Object(None).to_stub().unwrap(), "object");
        assert_eq!(
            DataType::Object(Some("Foo\\Bar")).to_stub().unwrap(),
            "\\Foo\\Bar"
        );
        assert_eq!(DataType::Resource.to_stub().unwrap(), "resource");
        assert_eq!(DataType::Callable.to_stub().unwrap(), "callable");
        assert_eq!(DataType::Iterable.to_stub().unwrap(), "iterable");
        assert_eq!(DataType::Mixed.to_stub().unwrap(), "mixed");
        assert_eq!(DataType::Undef.to_stub().unwrap(), "mixed");
        assert_eq!(DataType::Ptr.to_stub().unwrap(), "mixed");
        assert_eq!(DataType::Indirect.to_stub().unwrap(), "mixed");
        assert_eq!(DataType::ConstantExpression.to_stub().unwrap(), "mixed");
        assert_eq!(DataType::Reference.to_stub().unwrap(), "reference");
    }

    #[test]
    fn test_parse_rustdoc() {
        use super::{Str, parse_rustdoc};

        // Test basic rustdoc parsing
        let docs: Vec<Str> = vec![
            " Gives you a nice greeting!".into(),
            "".into(),
            " # Arguments".into(),
            "".into(),
            " * `name` - Your name".into(),
            " * `age` - Your age".into(),
            "".into(),
            " # Returns".into(),
            "".into(),
            " Nice greeting!".into(),
        ];

        let parsed = parse_rustdoc(&docs);

        // Check summary
        assert_eq!(parsed.summary.len(), 2);
        assert!(parsed.summary[0].contains("Gives you a nice greeting"));

        // Check params
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params.get("name"), Some(&"Your name".to_string()));
        assert_eq!(parsed.params.get("age"), Some(&"Your age".to_string()));

        // Check returns
        assert!(parsed.returns.is_some());
        assert!(
            parsed
                .returns
                .as_ref()
                .is_some_and(|r| r.contains("Nice greeting"))
        );
    }

    #[test]
    fn test_parse_param_line() {
        use super::parse_param_line;

        // Test backtick format
        assert_eq!(
            parse_param_line("`name` - Your name"),
            Some(("name".to_string(), "Your name".to_string()))
        );

        // Test with $ prefix
        assert_eq!(
            parse_param_line("`$name` - Your name"),
            Some(("name".to_string(), "Your name".to_string()))
        );

        // Test simple format
        assert_eq!(
            parse_param_line("name - Your name"),
            Some(("name".to_string(), "Your name".to_string()))
        );

        // Test invalid format
        assert_eq!(parse_param_line("no separator here"), None);
    }

    #[test]
    fn test_format_phpdoc() {
        use super::{DocBlock, Parameter, Retval, Str, format_phpdoc};
        use crate::describe::abi::Option;
        use crate::flags::DataType;

        // Create a DocBlock with rustdoc content
        let docs = DocBlock(
            vec![
                Str::from(" Greets the user."),
                Str::from(""),
                Str::from(" # Arguments"),
                Str::from(""),
                Str::from(" * `name` - The name to greet"),
                Str::from(""),
                Str::from(" # Returns"),
                Str::from(""),
                Str::from(" A greeting string."),
            ]
            .into(),
        );

        let params = vec![Parameter {
            name: "name".into(),
            ty: Option::Some(DataType::String),
            nullable: false,
            variadic: false,
            default: Option::None,
        }];

        let retval = Retval {
            ty: DataType::String,
            nullable: false,
        };

        let mut buf = String::new();
        format_phpdoc(&docs, &params, Some(&retval), &mut buf).expect("format_phpdoc failed");

        // Check that PHPDoc format is produced
        assert!(buf.contains("/**"));
        assert!(buf.contains("*/"));
        assert!(buf.contains("@param string $name The name to greet"));
        assert!(buf.contains("@return string A greeting string."));
        // Should NOT contain rustdoc section headers
        assert!(!buf.contains("# Arguments"));
        assert!(!buf.contains("# Returns"));
    }
}
