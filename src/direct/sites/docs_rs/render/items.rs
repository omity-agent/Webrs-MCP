#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc item variants are inspected through shared document references."
)]
use super::{Renderer, signatures};
use crate::{Result, error::AppError};
use rustdoc_types::{Id, Item, ItemEnum, Module};
mod declarations;
mod members;
pub(super) fn render_items(
    renderer: &mut Renderer<'_>,
    ids: &[Id],
    level: usize,
    associated: bool,
) -> Result<()> {
    for id in ids {
        if renderer.visited.contains(id) {
            continue;
        }
        let item = renderer.document.index.get(id).ok_or_else(|| {
            AppError::client(format!("Rustdoc JSON references missing item {}.", id.0))
        })?;
        if item.crate_id != renderer.local_crate_id && !matches!(&item.inner, ItemEnum::Use(_)) {
            continue;
        }
        renderer.visited.insert(*id);
        render_item(renderer, item, level, associated)?;
    }
    Ok(())
}
fn render_item(
    renderer: &mut Renderer<'_>,
    item: &Item,
    level: usize,
    associated: bool,
) -> Result<()> {
    match &item.inner {
        ItemEnum::Module(module) => declarations::module(renderer, item, module, level),
        ItemEnum::Struct(structure) => declarations::structure(renderer, item, structure, level),
        ItemEnum::Union(union) => declarations::union(renderer, item, union, level),
        ItemEnum::Enum(enumeration) => {
            declarations::enumeration(renderer, item, enumeration, level)
        }
        ItemEnum::Trait(trait_) => declarations::trait_(renderer, item, trait_, level),
        ItemEnum::TraitAlias(alias) => {
            declarations::trait_alias(renderer, item, alias, level);
            Ok(())
        }
        ItemEnum::Function(function) => {
            declaration(renderer, item, level, "function", |name| {
                signatures::function(item, name, function, associated)
            });
            Ok(())
        }
        ItemEnum::TypeAlias(alias) => {
            declaration(renderer, item, level, "type", |name| {
                signatures::type_alias(item, name, alias)
            });
            Ok(())
        }
        ItemEnum::Constant { type_, const_ } => {
            declaration(renderer, item, level, "constant", |name| {
                signatures::constant(item, name, type_, const_)
            });
            Ok(())
        }
        ItemEnum::Static(static_) => {
            declaration(renderer, item, level, "static", |name| {
                signatures::static_(item, name, static_)
            });
            Ok(())
        }
        ItemEnum::Use(use_) => {
            declaration(renderer, item, level, "re-export", |_| {
                signatures::use_(item, use_)
            });
            Ok(())
        }
        ItemEnum::Macro(source) => {
            declaration(renderer, item, level, "macro", |_| source.clone());
            Ok(())
        }
        ItemEnum::ProcMacro(macro_) => {
            declaration(renderer, item, level, "procedural macro", |name| {
                signatures::procedural_macro(item, name, macro_)
            });
            Ok(())
        }
        ItemEnum::Primitive(primitive) => {
            declaration(renderer, item, level, "primitive", |name| {
                let primitive_name = if name.is_empty() {
                    primitive.name.as_str()
                } else {
                    name
                };
                format!("pub primitive {primitive_name}")
            });
            Ok(())
        }
        ItemEnum::AssocConst {
            type_,
            value,
            default_unstable: _,
        } => {
            declaration(renderer, item, level, "associated constant", |name| {
                signatures::associated_constant(item, name, type_, value.as_deref())
            });
            Ok(())
        }
        ItemEnum::AssocType {
            generics,
            bounds,
            type_,
            default_unstable: _,
        } => {
            declaration(renderer, item, level, "associated type", |name| {
                signatures::associated_type(item, name, generics, bounds, type_.as_ref())
            });
            Ok(())
        }
        ItemEnum::Impl(implementation) => {
            members::implementation(renderer, item, implementation, level)
        }
        ItemEnum::Variant(_) => members::variant(renderer, item, level),
        ItemEnum::StructField(_)
        | ItemEnum::ExternCrate { name: _, rename: _ }
        | ItemEnum::ExternType => Ok(()),
    }
}
fn declaration(
    renderer: &mut Renderer<'_>,
    item: &Item,
    level: usize,
    kind: &str,
    signature: impl FnOnce(&str) -> String,
) {
    let name = item.name.as_deref().unwrap_or("<anonymous>");
    renderer.heading(level, &format!("{kind} `{name}`"));
    renderer.code(&signature(name));
    renderer.docs(item.docs.as_deref());
    renderer.deprecation(item);
}
pub(super) fn item_name(item: &Item) -> &str {
    item.name.as_deref().unwrap_or("<anonymous>")
}
pub(super) fn item_header(renderer: &mut Renderer<'_>, item: &Item, level: usize, kind: &str) {
    renderer.heading(level, &format!("{kind} `{}`", item_name(item)));
    renderer.docs(item.docs.as_deref());
    renderer.deprecation(item);
}
pub(super) fn module_children(
    renderer: &mut Renderer<'_>,
    module: &Module,
    level: usize,
) -> Result<()> {
    render_items(renderer, &module.items, level, false)
}
