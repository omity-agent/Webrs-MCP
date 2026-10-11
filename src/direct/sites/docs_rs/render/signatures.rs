mod associated;
mod functions;
mod support;
mod types;
pub(super) use associated::{
    associated_constant, associated_type, implementation, procedural_macro,
};
pub(super) use functions::{
    constant, enumeration, function, static_, structure, trait_, trait_alias, type_alias, union,
    use_,
};
pub(super) fn type_name(value: &rustdoc_types::Type) -> String {
    types::type_name(value)
}
