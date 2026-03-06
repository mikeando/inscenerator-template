/// Demonstrates whitespace-control markers: `{{-`, `-}}`, `{%-`, `-%}`.
///
/// A `-` on the *inside* of any tag strips all adjacent whitespace
/// (spaces, tabs, newlines) from the neighbouring literal text:
///
///   `{{-  expr }}` — strips whitespace *before* the output tag
///   `{{ expr  -}}` — strips whitespace *after* the output tag
///   `{%-  tag  %}` — strips whitespace *before* the block tag
///   `{%   tag -%}` — strips whitespace *after* the block tag
///
/// Both sides can be combined on the same tag.
use inscenerator_template::{Template, ctx, render};

fn main() {
    // ------------------------------------------------------------------ //
    // 1. Comma-separated list
    //
    // Without whitespace control each loop iteration starts on a new line
    // because the template source has a newline after the {% for %} tag.
    // Right-trim on the opening for-tag eats that newline so the items
    // flow directly inline.
    // ------------------------------------------------------------------ //
    let items = ctx! { "items": ["alpha", "beta", "gamma"] };

    // Without whitespace control:
    let plain = Template::parse("{% for item in items %}\n{{ item }}{% endfor %}").unwrap();
    let out = render(&plain, items.as_ref()).unwrap();
    println!("Plain:");
    println!("{:?}", out); // "\nalpha\nbeta\ngamma"

    // With -%} on the for-tag we eat the newline that follows it, so the
    // items are rendered without a leading blank line.  The separator is a
    // literal ", " inside the body followed by a conditional that suppresses
    // the trailing comma after the last item.
    let csv = Template::parse(concat!(
        "{% for item in items -%}\n",
        "{{ item }}",
        "{%- if item != 'gamma' %}, {% endif -%}\n",
        "{% endfor %}",
    ))
    .unwrap();
    let out = render(&csv, items.as_ref()).unwrap();
    println!("CSV: {:?}", out); // "alpha, beta, gamma"

    // ------------------------------------------------------------------ //
    // 2. Bullet list — one item per line, no blank lines around the loop
    //
    // The for-tag lives on its own template line; both left- and right-trim
    // prevent that line from adding a blank line to the output.
    // endfor is given right-trim only so the newline at the end of each
    // loop body is preserved as the item separator.
    // ------------------------------------------------------------------ //
    let tasks = ctx! {
        "tasks": [
            { "done": true,  "text": "design API" },
            { "done": false, "text": "write tests" },
            { "done": true,  "text": "publish crate" }
        ]
    };

    let bullet = Template::parse(concat!(
        "{%- for t in tasks -%}\n",
        "{% if t.done %}[x]{% else %}[ ]{% endif %} {{ t.text }}\n",
        "{% endfor -%}",
    ))
    .unwrap();
    let out = render(&bullet, tasks.as_ref()).unwrap();
    println!("\nTask list:");
    print!("{}", out);
    // [x] design API
    // [ ] write tests
    // [x] publish crate

    // ------------------------------------------------------------------ //
    // 3. Inline output — strip surrounding whitespace from {{ }}
    // ------------------------------------------------------------------ //
    let greeting = ctx! { "name": "Alice" };
    let tmpl = Template::parse("Hello,  {{- name -}}  !").unwrap();
    let out = render(&tmpl, greeting.as_ref()).unwrap();
    println!("\nInline: {:?}", out); // "Hello,Alice!"
}
