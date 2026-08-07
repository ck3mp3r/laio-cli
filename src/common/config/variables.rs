use miette::{Result, miette};
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Default)]
struct Accumulated {
    values: Vec<String>,
    forced_array: bool,
}

/// Splits a comma-separated string into items, honoring `\,` as an
/// escaped literal comma. Whitespace around each item is trimmed.
fn split_csv(input: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&',') => {
                current.push(',');
                chars.next(); // consume the comma
            }
            ',' => {
                items.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    items.push(current.trim().to_string());
    items
}

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
/// Returns an error if any string is not in "key=value" or key[]=a,b,c format.
pub fn parse_variables(vars: &[String]) -> Result<HashMap<String, Value>> {
    let mut accumulator: HashMap<String, Accumulated> = HashMap::new();

    for var in vars {
        let parts: Vec<&str> = var.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(miette!(
                "Invalid variable format: '{}'. Expected format: key=value",
                var
            ));
        }
        let raw_key = parts[0].trim();
        let raw_value = parts[1].trim();

        let (key, forced_array) = match raw_key.strip_suffix("[]") {
            Some(stripped) => (stripped.trim(), true),
            None => (raw_key, false),
        };

        if key.is_empty() {
            return Err(miette!(
                "Invalid variable: key cannot be empty in '{}'",
                var
            ));
        }

        let entry = accumulator.entry(key.to_string()).or_default();
        entry.forced_array |= forced_array;

        if forced_array {
            if raw_value.is_empty() {
                // key[]= with nothing after '=' means "empty list", not "list
                // containing one blank string". This is the one deliberate
                // exception to split_csv's "always N+1 items" rule.
            } else {
                entry.values.extend(split_csv(raw_value));
            }
        } else {
            entry.values.push(raw_value.to_string());
        }
    }

    let mut map = HashMap::new();
    for (key, acc) in accumulator {
        let value = if acc.forced_array || acc.values.len() > 1 {
            json!(acc.values)
        } else {
            json!(acc.values[0])
        };
        map.insert(key, value);
    }
    Ok(map)
}
