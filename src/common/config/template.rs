//! Template rendering for configuration files using Tera.
//!
//! This module provides a simple interface for rendering YAML templates with variables.

use miette::Result;
use serde_json::Value;
use std::collections::HashMap;
use tera::{Context, Tera};

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

    // Disable auto-escaping since we're rendering YAML, not HTML
    tera.autoescape_on(vec![]);

    // Build Tera context from the variables map
    let mut context = Context::new();
    for (key, value) in variables {
        context.insert(key, value);
    }

    // Render the template with the context
    tera.render_str(template, &context)
        .map_err(|e| miette::miette!("Template rendering failed: {}", e))
}
