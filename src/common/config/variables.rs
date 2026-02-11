use miette::{miette, Result};
use std::collections::HashMap;

/// Parses a vector of "key=value" strings into a HashMap.
///
/// # Arguments
///
/// * `vars` - A slice of strings in "key=value" format
///
/// # Returns
///
/// Returns a HashMap with the parsed key-value pairs.
///
/// # Errors
///
/// Returns an error if any string is not in "key=value" format.
///
/// # Examples
///
/// ```
/// use laio::common::config::variables::parse_variables;
///
/// let vars = vec!["name=test".to_string(), "path=/home/user".to_string()];
/// let result = parse_variables(&vars).unwrap();
/// assert_eq!(result.get("name"), Some(&"test".to_string()));
/// assert_eq!(result.get("path"), Some(&"/home/user".to_string()));
/// ```
pub fn parse_variables(vars: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();

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

        map.insert(key.to_string(), value.to_string());
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_variable() {
        let vars = vec!["name=test".to_string()];
        let result = parse_variables(&vars).unwrap();

        assert_eq!(result.get("name"), Some(&"test".to_string()));
    }

    #[test]
    fn test_parse_multiple_variables() {
        let vars = vec![
            "name=myproject".to_string(),
            "path=/home/user/dev".to_string(),
            "editor=vim".to_string(),
        ];
        let result = parse_variables(&vars).unwrap();

        assert_eq!(result.get("name"), Some(&"myproject".to_string()));
        assert_eq!(result.get("path"), Some(&"/home/user/dev".to_string()));
        assert_eq!(result.get("editor"), Some(&"vim".to_string()));
    }

    #[test]
    fn test_parse_empty_vec() {
        let vars: Vec<String> = vec![];
        let result = parse_variables(&vars).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_value_with_equals_sign() {
        let vars = vec!["url=https://example.com?foo=bar".to_string()];
        let result = parse_variables(&vars).unwrap();

        assert_eq!(
            result.get("url"),
            Some(&"https://example.com?foo=bar".to_string())
        );
    }

    #[test]
    fn test_parse_whitespace_trimmed() {
        let vars = vec!["  name  =  test  ".to_string()];
        let result = parse_variables(&vars).unwrap();

        assert_eq!(result.get("name"), Some(&"test".to_string()));
    }

    #[test]
    fn test_parse_invalid_no_equals() {
        let vars = vec!["invalid".to_string()];
        let result = parse_variables(&vars);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid variable format"));
    }

    #[test]
    fn test_parse_invalid_empty_key() {
        let vars = vec!["=value".to_string()];
        let result = parse_variables(&vars);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("key cannot be empty"));
    }

    #[test]
    fn test_parse_empty_value_allowed() {
        let vars = vec!["name=".to_string()];
        let result = parse_variables(&vars).unwrap();

        assert_eq!(result.get("name"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_override_duplicate_keys() {
        let vars = vec!["name=first".to_string(), "name=second".to_string()];
        let result = parse_variables(&vars).unwrap();

        // Last value wins
        assert_eq!(result.get("name"), Some(&"second".to_string()));
    }
}
