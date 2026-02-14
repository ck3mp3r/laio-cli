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
