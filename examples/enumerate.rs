//! Demonstrates the `enumerate(list)` built-in function.
//!
//! `enumerate(list)` wraps each item in a map with two keys:
//!   - `index` — 0-based integer position
//!   - `value` — the original list item
//!
//! This makes loop position available without magic `loop.*` variables,
//! using the same function mechanism as every other built-in.
//!
//! Run with: cargo run --example enumerate

use inscenerator_template::{Template, ctx, render};

// Numbered list with a comma-separated footer.
// Uses `index` to drive both numbering and the "last item" check.
const TEMPLATE: &str = "\
Menu
----
{% for e in enumerate(items) %}{{ Item e }}
{% endfor %}
Order any of the above {{ len(items) }} items.
";

// Fragment context is the enumerate entry map: keys are "index" and "value".
// value itself is a map with name and price.
const ITEM: &str = "\
{{ index }}. {{ value.name }} — ${{ value.price }}\
{% if index == 0 %} ★ popular{% endif %}";

fn main() {
    let mut tmpl = Template::parse(TEMPLATE).unwrap();
    tmpl.add_fragment("Item", ITEM).unwrap();

    let ctx = ctx! {
        "items": [
            { "name": "Espresso",   "price": "3.50" },
            { "name": "Flat White", "price": "4.00" },
            { "name": "Cold Brew",  "price": "4.50" }
        ]
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    print!("{}", output);
}
