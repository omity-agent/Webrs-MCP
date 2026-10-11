#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc member variants are inspected through shared document references."
)]
use super::super::{Renderer, signatures};
use super::{item_header, item_name, render_items};
use crate::{Result, error::AppError};
use rustdoc_types::{Id, Impl, Item, ItemEnum, StructKind, Variant, VariantKind};
pub(super) fn struct_fields(
    renderer: &mut Renderer<'_>,
    kind: &StructKind,
    level: usize,
) -> Result<()> {
    match kind {
        StructKind::Unit => Ok(()),
        StructKind::Tuple(fields) => tuple_fields(renderer, fields, level),
        StructKind::Plain {
            fields,
            has_stripped_fields,
        } => named_fields(renderer, fields, *has_stripped_fields, level),
    }
}
pub(super) fn named_fields(
    renderer: &mut Renderer<'_>,
    fields: &[Id],
    has_stripped_fields: bool,
    level: usize,
) -> Result<()> {
    if fields.is_empty() && !has_stripped_fields {
        return Ok(());
    }
    renderer.heading(level, "Fields");
    for field_id in fields {
        field(renderer, *field_id, None)?;
    }
    if has_stripped_fields {
        renderer.line("- Private or hidden fields omitted.");
        renderer.blank();
    }
    Ok(())
}
fn tuple_fields(renderer: &mut Renderer<'_>, fields: &[Option<Id>], level: usize) -> Result<()> {
    if fields.is_empty() {
        return Ok(());
    }
    renderer.heading(level, "Fields");
    for (field_index, maybe_field_id) in fields.iter().enumerate() {
        if let Some(field_id) = maybe_field_id {
            field(renderer, *field_id, Some(field_index))?;
        } else {
            renderer.line(&format!("- field `{field_index}`: private or hidden"));
            renderer.blank();
        }
    }
    Ok(())
}
fn field(renderer: &mut Renderer<'_>, id: Id, index: Option<usize>) -> Result<()> {
    let item = renderer.document.index.get(&id).ok_or_else(|| {
        AppError::client(format!("Rustdoc JSON references missing field {}.", id.0))
    })?;
    let ItemEnum::StructField(type_) = &item.inner else {
        return Err(AppError::client(format!(
            "Rustdoc JSON item {} is not a struct field.",
            id.0
        )));
    };
    let name = index.map_or_else(
        || item_name(item).to_owned(),
        |field_index| field_index.to_string(),
    );
    let visibility = matches!(&item.visibility, rustdoc_types::Visibility::Public)
        .then_some("pub ")
        .unwrap_or_default();
    renderer.line(&format!(
        "- `{visibility}{name}`: `{}`",
        signatures::type_name(type_)
    ));
    renderer.docs(item.docs.as_deref());
    Ok(())
}
pub(super) fn variants(
    renderer: &mut Renderer<'_>,
    ids: &[Id],
    has_stripped_variants: bool,
    level: usize,
) -> Result<()> {
    if ids.is_empty() && !has_stripped_variants {
        return Ok(());
    }
    renderer.heading(level, "Variants");
    for id in ids {
        let item = renderer.document.index.get(id).ok_or_else(|| {
            AppError::client(format!("Rustdoc JSON references missing variant {}.", id.0))
        })?;
        variant(renderer, item, level + 1)?;
    }
    if has_stripped_variants {
        renderer.line("- Private or hidden variants omitted.");
        renderer.blank();
    }
    Ok(())
}
pub(super) fn variant(renderer: &mut Renderer<'_>, item: &Item, level: usize) -> Result<()> {
    let ItemEnum::Variant(variant) = &item.inner else {
        return Err(AppError::client(format!(
            "Rustdoc JSON item {} is not an enum variant.",
            item.id.0
        )));
    };
    item_header(renderer, item, level, "variant");
    renderer.code(&variant_signature(renderer, item, variant)?);
    match &variant.kind {
        VariantKind::Plain => Ok(()),
        VariantKind::Tuple(fields) => tuple_fields(renderer, fields, level + 1),
        VariantKind::Struct {
            fields,
            has_stripped_fields,
        } => named_fields(renderer, fields, *has_stripped_fields, level + 1),
    }
}
fn variant_signature(renderer: &Renderer<'_>, item: &Item, variant: &Variant) -> Result<String> {
    let name = item_name(item);
    let signature = match &variant.kind {
        VariantKind::Plain => name.to_owned(),
        VariantKind::Tuple(fields) => {
            let types = fields
                .iter()
                .map(|_| "_".to_owned())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({types})")
        }
        VariantKind::Struct {
            fields,
            has_stripped_fields: _,
        } => {
            let names = fields
                .iter()
                .map(|field_id| {
                    let field = renderer.document.index.get(field_id).ok_or_else(|| {
                        AppError::client(format!(
                            "Rustdoc JSON references missing variant field {}.",
                            field_id.0
                        ))
                    })?;
                    Ok(format!("{}: _", item_name(field)))
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            format!("{name} {{ {names} }}")
        }
    };
    Ok(signature)
}
pub(super) fn implementations(renderer: &mut Renderer<'_>, ids: &[Id], level: usize) -> Result<()> {
    for id in ids {
        if renderer.visited.contains(id) {
            continue;
        }
        let item = renderer.document.index.get(id).ok_or_else(|| {
            AppError::client(format!("Rustdoc JSON references missing impl {}.", id.0))
        })?;
        let ItemEnum::Impl(implementation_data) = &item.inner else {
            return Err(AppError::client(format!(
                "Rustdoc JSON item {} is not an impl block.",
                id.0
            )));
        };
        if implementation_data.is_synthetic {
            continue;
        }
        renderer.visited.insert(*id);
        implementation(renderer, item, implementation_data, level)?;
    }
    Ok(())
}
pub(super) fn implementation(
    renderer: &mut Renderer<'_>,
    item: &Item,
    implementation: &Impl,
    level: usize,
) -> Result<()> {
    renderer.heading(
        level,
        &format!("`{}`", signatures::implementation(implementation)),
    );
    renderer.docs(item.docs.as_deref());
    render_items(renderer, &implementation.items, level + 1, true)
}
