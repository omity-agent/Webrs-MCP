#![expect(
    clippy::pattern_type_mismatch,
    reason = "Rustdoc generic variants are inspected through shared schema references."
)]
#![expect(
    clippy::rest_pattern_accessible_field,
    reason = "Rustdoc schema additions should not make generic rendering brittle."
)]
use super::{path_name, type_name};
use rustdoc_types::{
    AssocItemConstraintKind, GenericArg, GenericArgs, GenericBound, GenericParamDef,
    GenericParamDefKind, PreciseCapturingArg, Term, WherePredicate,
};
pub(super) fn generic_args(generic_arguments: &GenericArgs) -> String {
    match generic_arguments {
        GenericArgs::AngleBracketed {
            args: argument_list,
            constraints,
        } => {
            let mut values = argument_list.iter().map(argument).collect::<Vec<_>>();
            values.extend(constraints.iter().map(|constraint| {
                let binding = match &constraint.binding {
                    AssocItemConstraintKind::Equality(term) => format!(" = {}", term_name(term)),
                    AssocItemConstraintKind::Constraint(bounds) => {
                        format!(
                            ": {}",
                            bounds
                                .iter()
                                .map(bound_name)
                                .collect::<Vec<_>>()
                                .join(" + ")
                        )
                    }
                };
                format!("{}{}", constraint.name, binding)
            }));
            format!("<{}>", values.join(", "))
        }
        GenericArgs::Parenthesized { inputs, output } => {
            let mut result = format!(
                "({})",
                inputs.iter().map(type_name).collect::<Vec<_>>().join(", ")
            );
            if let Some(output_type) = output {
                result.push_str(" -> ");
                result.push_str(&type_name(output_type));
            }
            result
        }
        GenericArgs::ReturnTypeNotation => "(..)".to_owned(),
    }
}
pub(crate) fn generic_list(generics: &rustdoc_types::Generics) -> String {
    if generics.where_predicates.is_empty() {
        return String::new();
    }
    format!(
        " where {}",
        generics
            .where_predicates
            .iter()
            .map(predicate)
            .collect::<Vec<_>>()
            .join(", ")
    )
}
fn argument(argument: &GenericArg) -> String {
    match argument {
        GenericArg::Lifetime(value) => value.clone(),
        GenericArg::Type(value) => type_name(value),
        GenericArg::Const(value) => value.value.as_deref().unwrap_or(&value.expr).to_owned(),
        GenericArg::Infer => "_".to_owned(),
    }
}
pub(super) fn parameter(parameter: &GenericParamDef) -> String {
    match &parameter.kind {
        GenericParamDefKind::Lifetime { outlives } => {
            let mut value = parameter.name.clone();
            if !outlives.is_empty() {
                value.push_str(": ");
                value.push_str(&outlives.join(" + "));
            }
            value
        }
        GenericParamDefKind::Type {
            bounds,
            default: default_type,
            is_synthetic: _,
        } => {
            let mut value = parameter.name.clone();
            if !bounds.is_empty() {
                value.push_str(": ");
                value.push_str(
                    &bounds
                        .iter()
                        .map(bound_name)
                        .collect::<Vec<_>>()
                        .join(" + "),
                );
            }
            if let Some(default_type_value) = default_type {
                value.push_str(" = ");
                value.push_str(&type_name(default_type_value));
            }
            value
        }
        GenericParamDefKind::Const {
            type_,
            default: default_value,
        } => {
            let mut value = format!("const {}: {}", parameter.name, type_name(type_));
            if let Some(default_value_text) = default_value {
                value.push_str(" = ");
                value.push_str(default_value_text);
            }
            value
        }
    }
}
fn predicate(predicate: &WherePredicate) -> String {
    match predicate {
        WherePredicate::BoundPredicate {
            type_,
            bounds,
            generic_params,
        } => {
            let prefix = if generic_params.is_empty() {
                String::new()
            } else {
                "for<...> ".to_owned()
            };
            format!(
                "{prefix}{}: {}",
                type_name(type_),
                bounds
                    .iter()
                    .map(bound_name)
                    .collect::<Vec<_>>()
                    .join(" + ")
            )
        }
        WherePredicate::LifetimePredicate { lifetime, outlives } => {
            format!("{lifetime}: {}", outlives.join(" + "))
        }
        WherePredicate::EqPredicate { lhs, rhs } => {
            format!("{} = {}", type_name(lhs), term_name(rhs))
        }
    }
}
pub(super) fn bound_name(bound: &GenericBound) -> String {
    match bound {
        GenericBound::TraitBound { trait_, .. } => path_name(trait_),
        GenericBound::Outlives(value) => value.clone(),
        GenericBound::Use(arguments) => format!(
            "use<{}>",
            arguments
                .iter()
                .map(precise_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
fn precise_name(argument: &PreciseCapturingArg) -> String {
    match argument {
        PreciseCapturingArg::Lifetime(value) | PreciseCapturingArg::Param(value) => value.clone(),
    }
}
fn term_name(term: &Term) -> String {
    match term {
        Term::Type(type_) => type_name(type_),
        Term::Constant(value) => value.value.as_deref().unwrap_or(&value.expr).to_owned(),
    }
}
