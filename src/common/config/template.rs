//! Template rendering for configuration files using Tera.
//!
//! This module provides a simple interface for rendering YAML templates with variables.

use miette::Result;
use std::collections::HashMap;
use tera::{Context, Tera};

/// Renders a template string with the provided variables.
///
/// # Arguments
///
/// * `template` - The template string to render
/// * `variables` - A map of variable names to their values
///
/// # Returns
///
/// Returns the rendered string with all variables expanded.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use laio::common::config::template::render;
///
/// let mut vars = HashMap::new();
/// vars.insert("name", "my-session");
/// vars.insert("path", "/home/user/project");
///
/// let template = r#"
/// name: {{ name }}
/// path: {{ path | default(value=".") }}
/// "#;
///
/// let result = render(template, &vars).unwrap();
/// assert!(result.contains("my-session"));
/// ```
pub fn render(template: &str, variables: &HashMap<&str, &str>) -> Result<String> {
    // Create a one-time Tera instance
    let mut tera = Tera::default();

    // Disable auto-escaping since we're rendering YAML, not HTML
    tera.autoescape_on(vec![]);

    // Build Tera context from the variables map
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(*key, value);
    }

    // Render the template with the context
    tera.render_str(template, &context)
        .map_err(|e| miette::miette!("Template rendering failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable_substitution() {
        let mut vars = HashMap::new();
        vars.insert("name", "test-session");

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
        vars.insert("name", "my-project");
        vars.insert("path", "/home/user/dev");

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
        vars.insert("name", "work");
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
}
