//! Demonstrates the built-in string transformation functions:
//! `upper()`, `lower()`, `trim()`, and `replace()`.
//!
//! Scenario: a simple product listing where titles are normalised from
//! raw user input — mixed case, extra whitespace, slug-style names.
//!
//! Run with: cargo run --example string_builtins

use inscenerator_template::{Template, ctx, render};

const TEMPLATE: &str = "\
Products
========
{% for p in products %}{{ Row p }}
{% endfor %}";

// The fragment receives each product map as its context.
// Demonstrates upper(), lower(), trim(), and replace() in combination.
const ROW: &str = "\
- {{ upper(trim(name)) }}: ${{ price }}
  slug: {{ replace(lower(trim(name)), ' ', '-') }}";

fn main() {
    let mut tmpl = Template::parse(TEMPLATE).unwrap();
    tmpl.add_fragment("Row", ROW).unwrap();

    let ctx = ctx! {
        "products": [
            { "name": "  Widget Pro  ", "price": "9.99" },
            { "name": "SUPER Gadget",   "price": "24.99" },
            { "name": "basic tool",     "price": "4.50" }
        ]
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    print!("{}", output);
}
