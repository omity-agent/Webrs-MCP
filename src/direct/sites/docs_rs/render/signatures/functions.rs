#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc signature variants are inspected through shared schema references."
)]
use super::{
    support::{abi, bounds, visibility},
    types::{generic_list, generic_suffix},
};
use rustdoc_types::{Constant, Function, Item, TraitAlias, Type, TypeAlias, Use};
pub(crate) fn function(item: &Item, name: &str, function: &Function, associated: bool) -> String {
    let mut signature = visibility(item, associated).to_owned();
    if function.header.is_const {
        signature.push_str("const ");
    }
    if function.header.is_unsafe {
        signature.push_str("unsafe ");
    }
    if function.header.is_async {
        signature.push_str("async ");
    }
    signature.push_str(&abi(&function.header.abi));
    signature.push_str("fn ");
    signature.push_str(name);
    signature.push_str(&generic_suffix(&function.generics));
    let arguments = function
        .sig
        .inputs
        .iter()
        .map(|(argument_name, argument_type)| {
            format!("{argument_name}: {}", super::type_name(argument_type))
        })
        .chain(function.sig.is_c_variadic.then_some("...".to_owned()))
        .collect::<Vec<_>>()
        .join(", ");
    signature.push('(');
    signature.push_str(&arguments);
    signature.push(')');
    if let Some(output) = function.sig.output.as_ref() {
        signature.push_str(" -> ");
        signature.push_str(&super::type_name(output));
    }
    signature.push_str(&generic_list(&function.generics));
    signature
}
pub(crate) fn structure(item: &Item, name: &str, generics: &rustdoc_types::Generics) -> String {
    format!(
        "{}struct {name}{}",
        visibility(item, false),
        generic_suffix(generics)
    )
}
pub(crate) fn enumeration(item: &Item, name: &str, generics: &rustdoc_types::Generics) -> String {
    format!(
        "{}enum {name}{}",
        visibility(item, false),
        generic_suffix(generics)
    )
}
pub(crate) fn union(item: &Item, name: &str, generics: &rustdoc_types::Generics) -> String {
    format!(
        "{}union {name}{}",
        visibility(item, false),
        generic_suffix(generics)
    )
}
pub(crate) fn trait_(item: &Item, name: &str, trait_: &rustdoc_types::Trait) -> String {
    let mut signature = visibility(item, false).to_owned();
    if trait_.is_unsafe {
        signature.push_str("unsafe ");
    }
    if trait_.is_auto {
        signature.push_str("auto ");
    }
    signature.push_str("trait ");
    signature.push_str(name);
    signature.push_str(&generic_suffix(&trait_.generics));
    if !trait_.bounds.is_empty() {
        signature.push_str(": ");
        signature.push_str(&bounds(&trait_.bounds));
    }
    signature.push_str(&generic_list(&trait_.generics));
    signature
}
pub(crate) fn trait_alias(item: &Item, name: &str, alias: &TraitAlias) -> String {
    format!(
        "{}trait {name}{} = {};",
        visibility(item, false),
        generic_suffix(&alias.generics),
        bounds(&alias.params)
    )
}
pub(crate) fn type_alias(item: &Item, name: &str, alias: &TypeAlias) -> String {
    format!(
        "{}type {name}{} = {};",
        visibility(item, false),
        generic_suffix(&alias.generics),
        super::type_name(&alias.type_)
    )
}
pub(crate) fn constant(item: &Item, name: &str, type_: &Type, value: &Constant) -> String {
    format!(
        "{}const {name}: {} = {};",
        visibility(item, false),
        super::type_name(type_),
        value.value.as_deref().unwrap_or(&value.expr)
    )
}
pub(crate) fn static_(item: &Item, name: &str, static_: &rustdoc_types::Static) -> String {
    let mut signature = visibility(item, false).to_owned();
    if static_.is_mutable {
        signature.push_str("static mut ");
    } else {
        signature.push_str("static ");
    }
    signature.push_str(name);
    signature.push_str(": ");
    signature.push_str(&super::type_name(&static_.type_));
    signature.push_str(" = ");
    signature.push_str(&static_.expr);
    signature.push(';');
    signature
}
pub(crate) fn use_(item: &Item, use_: &Use) -> String {
    let mut signature = visibility(item, false).to_owned();
    signature.push_str("use ");
    signature.push_str(&use_.source);
    if use_.is_glob {
        signature.push_str("::*");
    } else if use_.name != use_.source.rsplit("::").next().unwrap_or_default() {
        signature.push_str(" as ");
        signature.push_str(&use_.name);
    }
    signature.push(';');
    signature
}
