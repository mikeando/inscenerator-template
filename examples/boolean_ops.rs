//! Demonstrates boolean `and` / `or` operators in template conditionals.
//!
//! Without these operators, multi-condition checks require awkward nesting.
//! With them, permission and role logic reads naturally in the template.
//!
//! Run with: cargo run --example boolean_ops

use inscenerator_template::{Template, ctx, render};

const TEMPLATE: &str = "\
Users
=====
{% for user in users %}{{ Row user }}
{% endfor %}";

// The fragment uses `and` / `or` to express compound access rules.
const ROW: &str = "\
{{ name }}: \
{% if role == 'admin' or role == 'editor' %}\
  can edit\
{% else %}\
  read-only\
{% endif %}\
{% if active and verified %}\
  [verified]\
{% endif %}";

fn main() {
    let mut tmpl = Template::parse(TEMPLATE).unwrap();
    tmpl.add_fragment("Row", ROW).unwrap();

    let ctx = ctx! {
        "users": [
            { "name": "Alice", "role": "admin",  "active": true,  "verified": true  },
            { "name": "Bob",   "role": "editor", "active": true,  "verified": false },
            { "name": "Carol", "role": "viewer", "active": true,  "verified": true  },
            { "name": "Dave",  "role": "viewer", "active": false, "verified": false }
        ]
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    print!("{}", output);
}
