//! Conventional-commits changelog generator.
//!
//! Demonstrates:
//! - The `ctx!` macro for building nested inline data (owned `Value::Map` entries)
//! - `starts_with()` and `contains()` built-in string functions
//! - Custom function registration
//! - Fragments for reusable entry formatting
//! - `len()` for a summary count
//!
//! Run with: cargo run --example changelog

use inscenerator_template::{Template, Value, ctx, render};

// --- Templates ------------------------------------------------------------

const TEMPLATE: &str = "\
# Changelog — v{{ version }} ({{ date }})
{% if starts_with(version, '0') %}
  ⚠ Pre-release: API may change between minor versions.
{% endif %}
{% for entry in entries %}{{ Entry entry }}
{% endfor %}
{{ len(entries) }} change(s) in this release.
";

// The fragment receives each entry map as its context.
// `badge(type)` is a custom function; `contains` is a built-in.
const ENTRY: &str = "\
{{ badge(type) }} {{ description }}\
{% if contains(description, 'BREAKING') %} ⚠{% endif %}";

// --- Entry point ----------------------------------------------------------

fn main() {
    let mut tmpl = Template::parse(TEMPLATE).unwrap();
    tmpl.add_fragment("Entry", ENTRY).unwrap();

    // Custom function: format a conventional-commit type as a fixed-width badge.
    tmpl.add_function("badge", |args: &[Value]| match args {
        [Value::Str(s)] => Ok(Value::from(if s.starts_with("feat") {
            "[feat ]"
        } else if s.starts_with("fix") {
            "[ fix ]"
        } else if s.starts_with("docs") {
            "[docs ]"
        } else {
            "[other]"
        })),
        _ => Err("badge() expects one Str argument".to_string()),
    });

    // All data is built inline — no struct definitions needed.
    let ctx = ctx! {
        "version": "0.2.0",
        "date": "2026-02-27",
        "entries": [
            {
                "type": "feat",
                "description": "Add comparison operators (==, !=, <, >, <=, >=)"
            },
            {
                "type": "feat",
                "description": "Add built-in functions: len, starts_with, ends_with, contains"
            },
            {
                "type": "feat",
                "description": "Add custom function registration via add_function()"
            },
            {
                "type": "feat",
                "description": "BREAKING: Rework Value type (Map/DataSource/Fn split)"
            },
            {
                "type": "fix",
                "description": "Expression errors now reported at Template::parse() time"
            },
            {
                "type": "docs",
                "description": "Add report and changelog examples"
            }
        ]
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    print!("{}", output);
}
