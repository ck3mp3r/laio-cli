use super::template::render;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_simple_variable_substitution() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), json!("test-session"));

    let template = "name: {{ name }}";
    let result = render(template, &vars).unwrap();

    assert_eq!(result, "name: test-session");
}

#[test]
fn test_variable_with_default() {
    let vars = HashMap::new(); // Empty - should use default

    let template = r#"name: {{ name | default(value="default-session") }}"#;
    let result = render(template, &vars).unwrap();

    assert_eq!(result, "name: default-session");
}

#[test]
fn test_multiple_variables() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), json!("my-project"));
    vars.insert("path".to_string(), json!("/home/user/dev"));

    let template = r#"
name: {{ name }}
path: {{ path }}
"#;
    let result = render(template, &vars).unwrap();

    assert!(result.contains("name: my-project"));
    assert!(result.contains("path: /home/user/dev"));
}

#[test]
fn test_missing_required_variable_fails() {
    let vars = HashMap::new();

    let template = "name: {{ required_var }}"; // No default
    let result = render(template, &vars);

    assert!(result.is_err());
}

#[test]
fn test_yaml_template_with_defaults() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), json!("work"));
    // path is NOT provided, should use default

    let template = r#"
name: {{ name }}
path: {{ path | default(value="~") }}
windows:
  - name: {{ window_name | default(value="code") }}
    panes:
      - flex: 1
"#;
    let result = render(template, &vars).unwrap();

    assert!(result.contains("name: work"));
    assert!(result.contains("path: ~"));
    assert!(result.contains("name: code"));
}

#[test]
fn test_array_variable_in_loop() {
    let mut vars = HashMap::new();
    vars.insert("projects".to_string(), json!(["web", "api", "cli"]));

    let template = r#"
{% for project in projects %}
  - name: {{ project }}
{% endfor %}
"#;
    let result = render(template, &vars).unwrap();

    assert!(result.contains("- name: web"));
    assert!(result.contains("- name: api"));
    assert!(result.contains("- name: cli"));
}

#[test]
fn test_as_array_wraps_plain_string() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!("solo"));
    let template = r#"
{% for item in items | as_array %}
  - name: {{ item }}
{% endfor %}
"#;
    let result = render(template, &vars).unwrap();
    assert!(result.contains("- name: solo"));
    // Exactly one iteration, not one per character.
    assert_eq!(result.matches("- name:").count(), 1);
}

#[test]
fn test_as_array_passes_through_existing_array_unchanged() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(["web", "api", "cli"]));
    let template = r#"
{% for item in items | as_array %}
  - name: {{ item }}
{% endfor %}
"#;
    let result = render(template, &vars).unwrap();
    assert!(result.contains("- name: web"));
    assert!(result.contains("- name: api"));
    assert!(result.contains("- name: cli"));
    assert_eq!(result.matches("- name:").count(), 3);
}

#[test]
fn test_as_array_preserves_array_order() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(["first", "second", "third"]));
    let template = r#"{% for item in items | as_array %}{{ item }},{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "first,second,third,");
}

#[test]
fn test_as_array_wraps_single_char_string_correctly() {
    // The exact regression case: a one-character string must not be
    // treated as "already iterable" by anything upstream of the filter.
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!("x"));
    let template = r#"{% for item in items | as_array %}[{{ item }}]{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "[x]");
}

#[test]
fn test_as_array_on_null_produces_empty_array() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(null));
    let template = r#"{% for item in items | as_array %}{{ item }},{% endfor %}done"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "done");
}

#[test]
fn test_as_array_on_empty_array_stays_empty() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!([]));
    let template = r#"{% for item in items | as_array %}{{ item }},{% endfor %}done"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "done");
}

#[test]
fn test_as_array_on_empty_string_wraps_as_single_blank_item() {
    // An empty string is still a scalar, not "nothing" - it wraps to
    // one item, unlike null/missing which produce zero items.
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(""));
    let template = r#"{% for item in items | as_array %}[{{ item }}]{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "[]");
}

#[test]
fn test_as_array_combined_with_default_for_missing_variable() {
    // The two-filter chain: missing variable -> default(value=[]) fires
    // -> as_array is a no-op on the resulting empty array.
    let vars = HashMap::new();
    let template =
        r#"{% for item in items | default(value=[]) | as_array %}{{ item }},{% endfor %}done"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "done");
}

#[test]
fn test_as_array_combined_with_default_for_present_string() {
    // default(value=[]) only fires on missing variables, so a present
    // string passes through to as_array, which wraps it.
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!("solo"));
    let template =
        r#"{% for item in items | default(value=[]) | as_array %}[{{ item }}]{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "[solo]");
}

#[test]
fn test_as_array_wraps_number() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(42));
    let template = r#"{% for item in items | as_array %}[{{ item }}]{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "[42]");
}

#[test]
fn test_as_array_wraps_bool() {
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!(true));
    let template = r#"{% for item in items | as_array %}[{{ item }}]{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "[true]");
}

#[test]
fn test_as_array_wraps_object_as_single_element() {
    // A JSON object is a scalar from the filter's perspective - it
    // should not be iterated key-by-key, just wrapped whole.
    let mut vars = HashMap::new();
    vars.insert("items".to_string(), json!({"name": "solo"}));
    let template = r#"{% for item in items | as_array %}{{ item.name }},{% endfor %}"#;
    let result = render(template, &vars).unwrap();
    assert_eq!(result, "solo,");
}

#[test]
fn test_as_array_in_full_yaml_template() {
    // End-to-end sanity check against the exact shape of template this
    // filter was written to protect - a for-loop building windows from
    // a variable that might arrive as a single unbracketed --var value.
    let mut vars = HashMap::new();
    vars.insert("microservice".to_string(), json!("auth"));
    let template = r#"
{% for ms in microservice | default(value=[]) | as_array %}
  - name: {{ ms }}
    flex_direction: row
{% endfor %}
"#;
    let result = render(template, &vars).unwrap();
    assert!(result.contains("- name: auth"));
    assert_eq!(result.matches("- name:").count(), 1);
}
