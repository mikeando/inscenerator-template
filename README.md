# inscenerator-template

A lightweight Rust text templating engine where your own types drive rendering — no JSON serialization required.

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

Expressions support dotted paths (`user.address.city`), negation (`!flag`), comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`), function calls (`ends_with(name, ".rs")`), and literals (`true`, `false`, `null`, integers, floats, `"strings"`, `'strings'`). All expressions are parsed at `Template::parse()` time — invalid expressions are caught immediately, not at render time.

### Comparison operators

| Operator | Types | Notes |
|----------|-------|-------|
| `==`, `!=` | `Int`, `Float`, `Str`, `Bool`, `Null` | `Int`/`Float` coerce automatically |
| `<`, `>`, `<=`, `>=` | `Int`, `Float` only | Error on strings, bools, etc. |

### Built-in functions

| Function | Signature | Returns |
|----------|-----------|---------|
| `len` | `Str` or `List` | `Int` — character count for strings, element count for lists |
| `starts_with` | `Str`, `Str` | `Bool` |
| `ends_with` | `Str`, `Str` | `Bool` |
| `contains` | `Str`, `Str` | `Bool` |

```
{% if ends_with(filename, '.rs') %}Rust file{% endif %}
{% if len(items) > 0 %}Has items{% endif %}
```

### Custom functions

Register functions on a `Template` with `add_function()`. Custom functions can shadow built-ins.

```rust
let mut tmpl = Template::parse("{{ greet(name) }}!").unwrap();
tmpl.add_function("greet", |args: &[Value]| match args {
    [Value::Str(s)] => Ok(Value::from(format!("Hello, {s}!"))),
    _ => Err("greet() expects one Str".to_string()),
});
```

Fragment calls are distinguished from plain output by the first word starting with an uppercase letter — no extra syntax required.

## Quick start

```rust
use inscenerator_template::{ctx, DataSource, Value, Template, render};

// Option 1: the ctx! macro for ad-hoc inline data
let tmpl = Template::parse("Hello, {{ name }}!").unwrap();
let ctx  = ctx! { "name": "Alice" };
println!("{}", render(&tmpl, ctx.as_ref()).unwrap());
// → Hello, Alice!

// Option 2: implement DataSource on your own types — zero-copy, no serialization
#[derive(Debug)]
struct User { name: String, age: i64 }

impl DataSource for User {
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "name" => Some(Value::from(self.name.as_str())),
            "age"  => Some(Value::Int(self.age)),
            _      => None,
        }
    }
}

let tmpl = Template::parse("{{ name }} is {{ age }}.").unwrap();
let user = User { name: "Bob".into(), age: 25 };
println!("{}", render(&tmpl, &user).unwrap());
// → Bob is 25.
```

## The `ctx!` macro

`ctx!` builds nested data inline without defining structs. The root call returns `Arc<dyn DataSource>` for passing to `render()`; nested `{ ... }` blocks produce owned `Value::Map` values.

```rust
let ctx = ctx! {
    "user": { "name": "Alice", "age": 30 },
    "tags": ["rust", "template"],
    "active": true
};
render(&tmpl, ctx.as_ref()).unwrap();
```

## The `Value` type

`Value` is the universal value type used throughout the engine:

| Variant | Contents | Equality |
|---------|----------|----------|
| `Null` | — | equal to itself |
| `Bool(bool)` | — | structural |
| `Int(i64)` | — | structural |
| `Float(f64)` | — | structural |
| `Str(String)` | — | structural |
| `List(Arc<Vec<Value>>)` | — | structural |
| `Map(HashMap<String, Value>)` | Owned, transparent | Structural — key/value comparison |
| `DataSource(Arc<dyn DataSource>)` | Opaque Rust type | Identity — same `Arc` allocation |
| `Fn(Arc<dyn Function>)` | Callable | Identity — same `Arc` allocation |

Use `Map` for data you construct programmatically or return from functions. Use `DataSource` for existing Rust types you want to expose without copying.

## The `DataSource` trait

```rust
pub trait DataSource: Send + Sync + Debug {
    fn get(&self, key: &str) -> Option<Value>;
}
```

Return `Some(value)` when the key exists, `None` when it doesn't. `Some(Value::Null)` represents an explicitly-null key (distinct from a missing key). `HashMap<String, Value>` implements `DataSource` automatically.

## Fragments

Fragments are reusable sub-templates — think components. Register them by name and invoke with `{{ FragmentName expr }}`, where `expr` evaluates to a `Value::Map` or `Value::DataSource`. The fragment renders against that value as its entire context and has no implicit access to the outer scope, so the data contract is always explicit.

```rust
let mut tmpl = Template::parse(r#"
    {% for p in products %}{{ ProductView p }}{% endfor %}
"#).unwrap();

tmpl.add_fragment("ProductView", "{{ name }}: ${{ price }}\n").unwrap();
```

Fragments can call other fragments. Recursion is limited to 20 levels to guard against infinite loops.

## Loops

```rust
// Items can be Value::Map (built with ctx!) or Value::DataSource (your Rust types)
let products = Value::List(Arc::new(vec![
    Value::DataSource(Arc::new(Product { name: "Widget", price: 9 })),
    Value::DataSource(Arc::new(Product { name: "Gadget", price: 14 })),
]));

// Dotted-path access works on both Map and DataSource items
let src = "{% for p in products %}{{ p.name }}: ${{ p.price }}\n{% endfor %}";

// Or delegate each item to a fragment
let src = "{% for p in products %}{{ ProductView p }}{% endfor %}";
```

The loop variable overlays the parent context, so outer variables remain accessible inside the loop body.

## Examples

```
cargo run --example report     # Employee report: DataSource on Rust structs
cargo run --example changelog  # Changelog: ctx! macro, string functions, custom functions
```

## Architecture

```
src/
  lib.rs         — public re-exports
  value.rs       — Value enum, DataSource/Function traits, ctx!/context! macros
  expr.rs        — Expr AST, BinOp enum, parse_expr() (recursive-descent parser)
  lexer.rs       — tokenizer; detects FragmentName calls at lex time
  parser.rs      — token stream → AST (Node enum), Template struct, built-in functions
  renderer.rs    — AST walker, eval(), apply_binop(), LoopContext, fragment dispatch
  integration.rs — end-to-end tests (compiled only under #[cfg(test)])
examples/
  report.rs      — employee report using DataSource on Rust structs
  changelog.rs   — changelog generator using ctx! macro and custom functions
```
