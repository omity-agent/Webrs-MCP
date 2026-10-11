use super::{
    support::visibility,
    types::{generic_list, generic_suffix},
};
use rustdoc_types::{GenericBound, Impl, Item, MacroKind, ProcMacro, Type};
pub(crate) fn implementation(implementation: &Impl) -> String {
    let mut signature = String::from("impl ");
    if implementation.is_unsafe {
        signature.push_str("unsafe ");
    }
    signature.push_str(&generic_suffix(&implementation.generics));
    if implementation.is_negative {
        signature.push('!');
    }
    if let Some(trait_) = implementation.trait_.as_ref() {
        signature.push_str(&super::type_name(&Type::ResolvedPath(trait_.clone())));
        signature.push_str(" for ");
    }
    signature.push_str(&super::type_name(&implementation.for_));
    signature.push_str(&generic_list(&implementation.generics));
    signature
}
pub(crate) fn associated_constant(
    item: &Item,
    name: &str,
    type_: &Type,
    constant_value: Option<&str>,
) -> String {
    let mut signature = format!(
        "{}const {name}: {}",
        visibility(item, true),
        super::type_name(type_)
    );
    if let Some(constant_value_text) = constant_value {
        signature.push_str(" = ");
        signature.push_str(constant_value_text);
    }
    signature.push(';');
    signature
}
pub(crate) fn associated_type(
    item: &Item,
    name: &str,
    generics: &rustdoc_types::Generics,
    bounds: &[GenericBound],
    associated_type_value: Option<&Type>,
) -> String {
    let mut signature = format!(
        "{}type {name}{}",
        visibility(item, true),
        generic_suffix(generics)
    );
    if !bounds.is_empty() {
        signature.push_str(": ");
        signature.push_str(&super::support::bounds(bounds));
    }
    if let Some(associated_type_text) = associated_type_value {
        signature.push_str(" = ");
        signature.push_str(&super::type_name(associated_type_text));
    }
    signature.push(';');
    signature
}
pub(crate) fn procedural_macro(item: &Item, name: &str, macro_: &ProcMacro) -> String {
    let prefix = match macro_.kind {
        MacroKind::Bang => "macro",
        MacroKind::Attr => "attribute macro",
        MacroKind::Derive => "derive macro",
    };
    format!("{}{} `{name}`", visibility(item, false), prefix)
}
