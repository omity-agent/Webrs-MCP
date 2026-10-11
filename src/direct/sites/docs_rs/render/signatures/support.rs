#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc bound variants are inspected through shared schema references."
)]
use rustdoc_types::{Abi, GenericBound, Item, Type};
pub(super) fn bounds(values: &[GenericBound]) -> String {
    values.iter().map(bound).collect::<Vec<_>>().join(" + ")
}
pub(super) const fn visibility(item: &Item, associated: bool) -> &'static str {
    if matches!(&item.visibility, rustdoc_types::Visibility::Public) || !associated {
        "pub "
    } else {
        ""
    }
}
pub(super) fn abi(abi: &Abi) -> String {
    match abi {
        Abi::Rust => String::new(),
        Abi::C { unwind } => extern_abi("C", *unwind),
        Abi::Cdecl { unwind } => extern_abi("cdecl", *unwind),
        Abi::Stdcall { unwind } => extern_abi("stdcall", *unwind),
        Abi::Fastcall { unwind } => extern_abi("fastcall", *unwind),
        Abi::Aapcs { unwind } => extern_abi("aapcs", *unwind),
        Abi::Win64 { unwind } => extern_abi("win64", *unwind),
        Abi::SysV64 { unwind } => extern_abi("sysv64", *unwind),
        Abi::System { unwind } => extern_abi("system", *unwind),
        Abi::Other(name) => format!("extern \"{name}\" "),
    }
}
fn extern_abi(name: &str, unwind: bool) -> String {
    format!("extern \"{name}{}\" ", if unwind { "-unwind" } else { "" })
}
pub(super) fn bound(value: &GenericBound) -> String {
    match value {
        GenericBound::TraitBound {
            trait_,
            generic_params,
            modifier,
        } => {
            let modifier_text = match modifier {
                rustdoc_types::TraitBoundModifier::None => "",
                rustdoc_types::TraitBoundModifier::Maybe => "?",
                rustdoc_types::TraitBoundModifier::MaybeConst => "~const ",
            };
            let higher_ranked = if generic_params.is_empty() {
                String::new()
            } else {
                format!(
                    "for<{}> ",
                    generic_params
                        .iter()
                        .map(|_| "_")
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            format!(
                "{higher_ranked}{modifier_text}{}",
                super::type_name(&Type::ResolvedPath(trait_.clone()))
            )
        }
        GenericBound::Outlives(lifetime) => lifetime.clone(),
        GenericBound::Use(arguments) => format!(
            "use<{}>",
            arguments.iter().map(|_| "_").collect::<Vec<_>>().join(", ")
        ),
    }
}
