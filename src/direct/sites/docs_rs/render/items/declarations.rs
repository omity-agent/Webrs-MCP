use super::super::{Renderer, signatures};
use super::{item_header, module_children};
use crate::Result;
use rustdoc_types::{Enum, Item, Module, Struct, Trait, TraitAlias, Union};
pub(super) fn module(
    renderer: &mut Renderer<'_>,
    item: &Item,
    module: &Module,
    level: usize,
) -> Result<()> {
    item_header(renderer, item, level, "module");
    module_children(renderer, module, level + 1)
}
pub(super) fn structure(
    renderer: &mut Renderer<'_>,
    item: &Item,
    structure: &Struct,
    level: usize,
) -> Result<()> {
    item_header(renderer, item, level, "struct");
    renderer.code(&signatures::structure(
        item,
        super::item_name(item),
        &structure.generics,
    ));
    super::members::struct_fields(renderer, &structure.kind, level + 1)?;
    super::members::implementations(renderer, &structure.impls, level + 1)
}
pub(super) fn union(
    renderer: &mut Renderer<'_>,
    item: &Item,
    union: &Union,
    level: usize,
) -> Result<()> {
    item_header(renderer, item, level, "union");
    renderer.code(&signatures::union(
        item,
        super::item_name(item),
        &union.generics,
    ));
    super::members::named_fields(
        renderer,
        &union.fields,
        union.has_stripped_fields,
        level + 1,
    )?;
    super::members::implementations(renderer, &union.impls, level + 1)
}
pub(super) fn enumeration(
    renderer: &mut Renderer<'_>,
    item: &Item,
    enumeration: &Enum,
    level: usize,
) -> Result<()> {
    item_header(renderer, item, level, "enum");
    renderer.code(&signatures::enumeration(
        item,
        super::item_name(item),
        &enumeration.generics,
    ));
    super::members::variants(
        renderer,
        &enumeration.variants,
        enumeration.has_stripped_variants,
        level + 1,
    )?;
    super::members::implementations(renderer, &enumeration.impls, level + 1)
}
pub(super) fn trait_(
    renderer: &mut Renderer<'_>,
    item: &Item,
    trait_: &Trait,
    level: usize,
) -> Result<()> {
    item_header(renderer, item, level, "trait");
    renderer.code(&signatures::trait_(item, super::item_name(item), trait_));
    super::render_items(renderer, &trait_.items, level + 1, true)
}
pub(super) fn trait_alias(
    renderer: &mut Renderer<'_>,
    item: &Item,
    alias: &TraitAlias,
    level: usize,
) {
    item_header(renderer, item, level, "trait alias");
    renderer.code(&signatures::trait_alias(
        item,
        super::item_name(item),
        alias,
    ));
}
