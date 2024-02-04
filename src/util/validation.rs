use std::slice::from_ref;

use serde_valid::validation::Error;
use serde_valid::validation::Errors;

pub(crate) fn stringify_validation_errors(errors: &Errors) -> String {
    process_errors(errors, "").join("\n")
}

fn process_errors(errors: &Errors, prefix: &str) -> Vec<String> {
    match errors {
        Errors::Array(array_errors) => {
            log::trace!("array_errors: {}", array_errors);
            array_errors
                .items
                .iter()
                .enumerate()
                .flat_map(|(field, err)| process_errors(err.1, &format!("{}[{}]", prefix, field)))
                .collect::<Vec<String>>()
                .into_iter()
                .chain(
                    array_errors
                        .errors
                        .iter()
                        .enumerate()
                        .flat_map(|(field, err)| {
                            process_error(
                                from_ref(err),
                                &format!("{}[{}]", prefix, field),
                            )
                        }),
                )
                .collect()
        }

        Errors::Object(object_errors) => {
            log::trace!("object_errors: {}", object_errors);
            object_errors
                .properties
                .iter()
                .flat_map(|(field, err)| process_errors(err, &format!("{}.{}", prefix, field)))
                .collect()
        }

        Errors::NewType(new_type_error) => {
            log::trace!("new_type_error: {:?}", new_type_error);
            process_error(new_type_error, prefix)
        }
    }
}

fn process_error(errors: &[Error], prefix: &str) -> Vec<String> {
    errors
        .iter()
        .flat_map(|err| vec![format!("{}: {}", prefix, err.to_string())])
        .collect()
}
