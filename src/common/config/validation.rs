use std::slice::from_ref;

use miette::Report;
use serde_valid::validation::{Error, Errors};

pub(crate) fn generate_report(errors: Option<&Errors>) -> Report {
    build_error_tree(errors, "session")
}

fn build_error_tree(errors: Option<&Errors>, prefix: &str) -> Report {
    match errors {
        Some(Errors::Array(array_errors)) => {
            let mut messages = vec![];

            for (field, (_, err)) in array_errors.items.iter().enumerate() {
                let nested_report = build_error_tree(Some(err), &format!("{prefix}[{field}]"));
                messages.push(format!("{nested_report}"));
            }

            for (field, err) in array_errors.errors.iter().enumerate() {
                let single_error_report =
                    build_single_error_report(from_ref(err), &format!("{prefix}[{field}]"));
                messages.push(format!("{single_error_report}"));
            }

            Report::msg(messages.join("\n"))
        }

        Some(Errors::Object(object_errors)) => {
            let messages = object_errors
                .properties
                .iter()
                .map(|(field, err)| build_error_tree(Some(err), &format!("{prefix}.{field}")))
                .map(|r| format!("{r}"))
                .collect::<Vec<_>>();

            Report::msg(messages.join("\n"))
        }

        Some(Errors::NewType(new_type_error)) => build_single_error_report(new_type_error, prefix),

        None => Report::msg("No errors found."),
    }
}

fn build_single_error_report(errors: &[Error], prefix: &str) -> Report {
    let messages = errors
        .iter()
        .map(|err| format!("{prefix}: {err}"))
        .collect::<Vec<_>>()
        .join("\n");

    Report::msg(messages)
}
