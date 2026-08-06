use serde_json::json;

use super::variables::parse_variables;

#[test]
fn test_parse_single_variable() {
    let vars = vec!["name=test".to_string()];
    let result = parse_variables(&vars).unwrap();

    assert_eq!(result.get("name").unwrap().as_str(), Some("test"));
}

#[test]
fn test_parse_multiple_variables() {
    let vars = vec![
        "name=myproject".to_string(),
        "path=/home/user/dev".to_string(),
        "editor=vim".to_string(),
    ];
    let result = parse_variables(&vars).unwrap();

    assert_eq!(result.get("name").unwrap().as_str(), Some("myproject"));
    assert_eq!(result.get("path").unwrap().as_str(), Some("/home/user/dev"));
    assert_eq!(result.get("editor").unwrap().as_str(), Some("vim"));
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
        result.get("url").unwrap().as_str(),
        Some("https://example.com?foo=bar")
    );
}

#[test]
fn test_parse_whitespace_trimmed() {
    let vars = vec!["  name  =  test  ".to_string()];
    let result = parse_variables(&vars).unwrap();

    assert_eq!(result.get("name").unwrap().as_str(), Some("test"));
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

    assert_eq!(result.get("name").unwrap().as_str(), Some(""));
}

#[test]
fn test_parse_duplicate_keys_creates_array() {
    let vars = vec!["project=web".to_string(), "project=api".to_string()];
    let result = parse_variables(&vars).unwrap();

    // Multiple values should create an array
    let value = result.get("project").unwrap();
    assert!(value.is_array());
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str(), Some("web"));
    assert_eq!(arr[1].as_str(), Some("api"));
}

#[test]
fn test_parse_three_values_creates_array() {
    let vars = vec![
        "item=first".to_string(),
        "item=second".to_string(),
        "item=third".to_string(),
    ];
    let result = parse_variables(&vars).unwrap();

    let value = result.get("item").unwrap();
    assert!(value.is_array());
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str(), Some("first"));
    assert_eq!(arr[1].as_str(), Some("second"));
    assert_eq!(arr[2].as_str(), Some("third"));
}

#[test]
fn test_parse_single_value_stays_string() {
    let vars = vec!["name=single".to_string()];
    let result = parse_variables(&vars).unwrap();

    // Single value should remain a string, not an array
    let value = result.get("name").unwrap();
    assert!(value.is_string());
    assert_eq!(value.as_str(), Some("single"));
}

#[test]
fn test_parse_mixed_single_and_array_values() {
    let vars = vec![
        "name=myproject".to_string(),
        "env=dev".to_string(),
        "env=staging".to_string(),
        "env=prod".to_string(),
        "path=/tmp".to_string(),
    ];
    let result = parse_variables(&vars).unwrap();

    // name and path should be strings
    assert!(result.get("name").unwrap().is_string());
    assert_eq!(result.get("name").unwrap().as_str(), Some("myproject"));
    assert!(result.get("path").unwrap().is_string());
    assert_eq!(result.get("path").unwrap().as_str(), Some("/tmp"));

    // env should be an array
    let env = result.get("env").unwrap();
    assert!(env.is_array());
    let arr = env.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str(), Some("dev"));
    assert_eq!(arr[1].as_str(), Some("staging"));
    assert_eq!(arr[2].as_str(), Some("prod"));
}

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
