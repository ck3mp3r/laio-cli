use miette::{miette, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Parses a vector of "key=value" strings into a HashMap.
///
/// When the same key appears multiple times, an array is automatically created.
/// Single values remain as strings for backward compatibility.
///
/// # Arguments
///
/// * `vars` - A slice of strings in "key=value" format
///
/// # Returns
///
/// Returns a HashMap with the parsed key-value pairs, where values can be
/// strings (single occurrence) or arrays (multiple occurrences).
///
/// # Errors
///
/// Returns an error if any string is not in "key=value" format.
pub fn parse_variables(vars: &[String]) -> Result<HashMap<String, Value>> {
    // First pass: accumulate all values for each key
    let mut accumulator: HashMap<String, Vec<String>> = HashMap::new();

    for var in vars {
        let parts: Vec<&str> = var.splitn(2, '=').collect();

        if parts.len() != 2 {
            return Err(miette!(
                "Invalid variable format: '{}'. Expected format: key=value",
                var
            ));
        }

        let key = parts[0].trim();
        let value = parts[1].trim();

        if key.is_empty() {
            return Err(miette!(
                "Invalid variable: key cannot be empty in '{}'",
                var
            ));
        }

        accumulator
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }

    // Second pass: convert to serde_json::Value
    // Single value -> string, Multiple values -> array
    let mut map = HashMap::new();
    for (key, values) in accumulator {
        let value = if values.len() == 1 {
            json!(values[0]) // Single value stays as string
        } else {
            json!(values) // Multiple values become array
        };
        map.insert(key, value);
    }

    Ok(map)
}
