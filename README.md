# inscenerator-template

A lightweight Rust text templating engine where your own types drive rendering — no JSON serialization required.

WARNING: The initial version is almost entierly one-shot vide coded. It probably does not work correctly in all cases, and
needs a heap more tests. But what tests thare are, actually pass.

## Template syntax

| Syntax | Purpose |
|---|---|
| `{{ expr }}` | Output a value |
| `{{ FragmentName expr }}` | Render a named fragment with `expr` as its context |
| `{% if expr %}...{% endif %}` | Conditional |
| `{% elif expr %}` | Else-if branch |
| `{% else %}` | Else branch |
| `{% for item in list %}...{% endfor %}` | Loop over a `Value::List` |
| `{# comment #}` | Ignored |

Expressions support dotted paths (`user.address.city`), negation (`!flag`), and literals (`true`, `false`, `null`, integers, floats, `"strings"`).

Fragment calls are distinguished from plain output by the first word starting with an uppercase letter — no extra syntax required.

## Quick start

```rust
use inscenerator_template::{context, DataSource, Value, Template, render};

// Option 1: use the context! macro for ad-hoc maps
let tmpl = Template::parse("Hello, {{ name }}!").unwrap();
let ctx  = context! { "name" => Value::from("Alice") };
println!("{}", render(&tmpl, ctx.as_ref()).unwrap());
// → Hello, Alice!

// Option 2: implement DataSource directly on your own types
#[derive(Debug)]
struct User { name: String, age: i64 }

impl DataSource for User {
    fn get(&self, key: &str) -> Value {
        match key {
            "name" => Value::from(self.name.as_str()),
            "age"  => Value::Int(self.age),
            _      => Value::Null,
        }
    }
}

let tmpl = Template::parse("{{ name }} is {{ age }}.").unwrap();
let user = User { name: "Bob".into(), age: 25 };
println!("{}", render(&tmpl, &user).unwrap());
// → Bob is 25.
```

## Fragments

Fragments are reusable sub-templates that act like components. You register them by name and invoke them with `{{ FragmentName expr }}`, where `expr` is any expression that resolves to a `Value::Map`. The fragment renders entirely against that map — it has no implicit access to the outer context, keeping the data contract explicit.

```rust
let mut tmpl = Template::parse(r#"
    {% for p in products %}
    {{ ProductView p }}
    {% endfor %}
"#).unwrap();

tmpl.add_fragment("ProductView", r#"
    Product: {{ name }}
    Price:   ${{ price }}
    ---
"#).unwrap();
```

`ProductView` only sees what `p` exposes — `name`, `price`, and whatever else the underlying `DataSource` provides. Nothing leaks in from the outer scope, so fragments are easy to reason about and test in isolation.

## Loops

List items can themselves be `Value::Map(Arc<dyn DataSource>)`, making fragment calls and dotted-path access work naturally inside loops:

```rust
let products = Value::List(Arc::new(vec![
    Value::Map(Arc::new(Product { name: "Widget", price: 9 })),
    Value::Map(Arc::new(Product { name: "Gadget", price: 14 })),
]));

// Plain dotted-path access
let src = "{% for p in products %}{{ p.name }}: ${{ p.price }}\n{% endfor %}";

// Or delegate to a fragment
let src = "{% for p in products %}{{ ProductView p }}{% endfor %}";
```

## Architecture

```
src/
  lib.rs       — public re-exports
  value.rs     — Value enum, DataSource trait, From impls
  lexer.rs     — tokenizer; detects FragmentName calls at lex time
  parser.rs    — token stream → AST (Node enum), Template struct
  renderer.rs  — AST walker, eval(), LoopContext and fragment dispatch
tests/
  integration.rs — end-to-end tests covering all features
```
