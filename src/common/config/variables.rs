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
            entry.values.extend(split_csv(raw_value));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_single_value_is_string() {
        let vars = vec!["key=alpha".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!("alpha"));
    }

    #[test]
    fn plain_repeated_key_is_array() {
        let vars = vec!["key=alpha".to_string(), "key=beta".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha", "beta"]));
    }

    #[test]
    fn bracket_single_value_is_array_of_one() {
        let vars = vec!["key[]=alpha".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha"]));
    }

    #[test]
    fn bracket_csv_splits_into_array() {
        let vars = vec!["key[]=alpha,beta,gamma,delta".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha", "beta", "gamma", "delta"]));
    }

    #[test]
    fn bracket_csv_repeated_flags_combine() {
        let vars = vec!["key[]=alpha,beta".to_string(), "key[]=gamma".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha", "beta", "gamma"]));
    }

    #[test]
    fn bracket_csv_escaped_comma() {
        let vars = vec![r"key[]=foo\,bar,baz".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["foo,bar", "baz"]));
    }

    #[test]
    fn mixing_plain_and_bracket_combines() {
        let vars = vec!["key=alpha".to_string(), "key[]=beta,gamma".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha", "beta", "gamma"]));
    }

    #[test]
    fn mixing_bracket_then_plain_combines() {
        let vars = vec!["key[]=alpha,beta".to_string(), "key=gamma".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!(["alpha", "beta", "gamma"]));
    }

    #[test]
    fn plain_value_with_comma_stays_literal() {
        let vars = vec!["key=foo,bar".to_string()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], json!("foo,bar"));
    }
}
