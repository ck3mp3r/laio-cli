//! Template rendering for configuration files using Tera.
//!
//! This module provides a simple interface for rendering YAML templates with variables.

use miette::Result;
use serde_json::Value;
use std::collections::HashMap;
use tera::{Context, Kwargs, State, Tera, Value as TeraValue, value::ValueKind};

/// Renders a template string with the provided variables.
///
/// # Arguments
///
/// * `template` - The template string to render
/// * `variables` - A map of variable names to their values (can be strings, arrays, objects)
///
/// # Returns
///
/// Returns the rendered string with all variables expanded.
pub fn render(template: &str, variables: &HashMap<String, Value>) -> Result<String> {
    // Create a one-time Tera instance
    let mut tera = Tera::default();
    tera.register_filter(
        "as_array",
        |value: &TeraValue, _: Kwargs, _: &State| match value.kind() {
            // Wraps a non-array value in a single-element array; arrays pass through
            // unchanged. Lets templates guard against a variable turning out to be a
            // plain string even when `--var key[]=...` wasn't used.
            ValueKind::Array => value.clone(),
            ValueKind::None => Vec::<TeraValue>::new().into(),
            _ => vec![value.clone()].into(),
        },
    );

    // Disable auto-escaping since we're rendering YAML, not HTML
    tera.autoescape_on(Vec::<&str>::new());

    // Build Tera context from the variables map
    let context = Context::from_serialize(variables)
        .map_err(|e| miette::miette!("Template context error: {}", e))?;

    // Render the template with the context
    tera.render_str(template, &context, false)
        .map_err(|e| miette::miette!("Template rendering failed: {}", e))
}
