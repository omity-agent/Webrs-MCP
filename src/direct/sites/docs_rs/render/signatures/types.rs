#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc type variants are inspected through shared schema references."
)]
#![expect(
    clippy::rest_pattern_accessible_field,
    reason = "Rustdoc schema additions should not make type rendering brittle."
)]
use rustdoc_types::{FunctionPointer, Path, Type};
mod generics;
use generics::generic_args;
pub(crate) use generics::generic_list;
pub(super) fn type_name(value: &Type) -> String {
    match value {
        Type::ResolvedPath(path) => path_name(path),
        Type::DynTrait(dynamic) => {
            let mut name = String::from("dyn ");
            name.push_str(
                &dynamic
                    .traits
                    .iter()
                    .map(|trait_| path_name(&trait_.trait_))
                    .collect::<Vec<_>>()
                    .join(" + "),
            );
            if let Some(lifetime) = dynamic.lifetime.as_deref() {
                name.push_str(" + ");
                name.push_str(lifetime);
            }
            name
        }
        Type::Generic(name) | Type::Primitive(name) => name.clone(),
        Type::FunctionPointer(function) => function_pointer(function),
        Type::Tuple(types) => format!(
            "({})",
            types.iter().map(type_name).collect::<Vec<_>>().join(", ")
        ),
        Type::Slice(type_) => format!("[{}]", type_name(type_)),
        Type::Array { type_, len } => format!("[{}; {len}]", type_name(type_)),
        Type::Pat { type_, .. } => type_name(type_),
        Type::ImplTrait(bounds) => format!(
            "impl {}",
            bounds
                .iter()
                .map(generics::bound_name)
                .collect::<Vec<_>>()
                .join(" + ")
        ),
        Type::Infer => "_".to_owned(),
        Type::RawPointer { is_mutable, type_ } => {
            format!(
                "*{} {}",
                if *is_mutable { "mut" } else { "const" },
                type_name(type_)
            )
        }
        Type::BorrowedRef {
            lifetime,
            is_mutable,
            type_,
        } => {
            let lifetime_text = lifetime
                .as_deref()
                .map_or(String::new(), |lifetime_value| format!("{lifetime_value} "));
            format!(
                "&{lifetime}{}{type_name}",
                if *is_mutable { "mut " } else { "" },
                lifetime = lifetime_text,
                type_name = type_name(type_)
            )
        }
        Type::QualifiedPath {
            name,
            args,
            self_type,
            trait_,
        } => {
            let mut result = String::from("<");
            result.push_str(&type_name(self_type));
            if let Some(associated_trait) = trait_ {
                result.push_str(" as ");
                result.push_str(&path_name(associated_trait));
            }
            result.push_str(">::");
            result.push_str(name);
            if let Some(generic_arguments) = args {
                result.push_str(&generic_args(generic_arguments));
            }
            result
        }
    }
}
pub(super) fn generic_suffix(generics: &rustdoc_types::Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }
    format!(
        "<{}>",
        generics
            .params
            .iter()
            .map(generics::parameter)
            .collect::<Vec<_>>()
            .join(", ")
    )
}
fn path_name(path: &Path) -> String {
    let mut name = path.path.clone();
    if let Some(args) = path.args.as_deref() {
        name.push_str(&generic_args(args));
    }
    name
}
fn function_pointer(function: &FunctionPointer) -> String {
    let mut signature = String::new();
    if !function.generic_params.is_empty() {
        signature.push_str("for<...> ");
    }
    signature.push_str("fn(");
    signature.push_str(
        &function
            .sig
            .inputs
            .iter()
            .map(|(_, type_)| type_name(type_))
            .collect::<Vec<_>>()
            .join(", "),
    );
    signature.push(')');
    if let Some(output) = function.sig.output.as_ref() {
        signature.push_str(" -> ");
        signature.push_str(&type_name(output));
    }
    signature
}
