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
| `{{- expr }}` / `{{ expr -}}` | Output, stripping whitespace before / after |
| `{%- tag %}` / `{% tag -%}` | Block tag, stripping whitespace before / after |

Expressions support dotted paths (`user.address.city`), negation (`!flag`), arithmetic (`+`, `-`, `*`, `/`, `%`), comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`), boolean operators (`and`/`or` or `&&`/`||`), function calls (`ends_with(name, ".rs")`), parenthesised grouping (`(a + b) * c`), and literals (`true`, `false`, `null`, integers, floats, `"strings"`, `'strings'`). All expressions are parsed at `Template::parse()` time — invalid expressions are caught immediately, not at render time.

### Arithmetic operators

| Operator | LHS / RHS | Result | Notes |
|----------|-----------|--------|-------|
| `+` | `Int`, `Float` (mixed ok) | `Int` or `Float` | Also `Str + Str` → concatenation |
| `-` | `Int`, `Float` (mixed ok) | `Int` or `Float` | Unary `-x` is also supported |
| `*` | `Int`, `Float` (mixed ok) | `Int` or `Float` | |
| `/` | `Int`, `Float` (mixed ok) | `Int` or `Float` | `Int / Int` truncates; errors on divide-by-zero |
| `%` | `Int`, `Int` | `Int` | Errors on modulo-by-zero |

Precedence (high → low): unary `-` → `*` `/` `%` → `+` `-` → comparisons → `and`/`or`.
Use `( expr )` to group sub-expressions.

```
{{ price * qty }}
{{ total / count }}
{{ index % 2 == 0 }}
{{ first + ' ' + last }}
{{ (a + b) * c }}
{{ -offset + base }}
```

### Comparison operators

| Operator | Types | Notes |
|----------|-------|-------|
| `==`, `!=` | `Int`, `Float`, `Str`, `Bool`, `Null` | `Int`/`Float` coerce automatically |
| `<`, `>`, `<=`, `>=` | `Int`, `Float` only | Error on strings, bools, etc. |

### Boolean operators

`and` / `or` (or `&&` / `||`) combine `Bool` expressions. Both operands must evaluate to `Bool`. Operators chain left-to-right and bind looser than comparisons, so `a == b and c == d` works as expected.

```
{% if role == 'admin' or role == 'editor' %}can edit{% endif %}
{% if active and verified %}[verified]{% endif %}
{% if a and b and c %}all three{% endif %}
```

### Built-in functions

#### String testing

| Function | Signature | Returns |
|----------|-----------|---------|
| `len` | `Str` or `List` | `Int` — character count for strings, element count for lists |
| `starts_with` | `Str`, `Str` | `Bool` |
| `ends_with` | `Str`, `Str` | `Bool` |
| `contains` | `Str`, `Str` | `Bool` |

#### String transformation

| Function | Signature | Returns |
|----------|-----------|---------|
| `upper(s)` | `Str` | `Str` — Unicode uppercase |
| `lower(s)` | `Str` | `Str` — Unicode lowercase |
| `trim(s)` | `Str` | `Str` — strips leading and trailing whitespace |
| `replace(s, from, to)` | `Str`, `Str`, `Str` | `Str` — replaces all occurrences of `from` with `to` |

#### List utilities

| Function | Signature | Returns |
|----------|-----------|---------|
| `enumerate(list)` | `List` | `List` of `Map`s with keys `"index"` (`Int`) and `"value"` (original item) |

```
{% if ends_with(filename, '.rs') %}Rust file{% endif %}
{% if len(items) > 0 %}Has items{% endif %}
{{ upper(trim(title)) }}
{{ replace(slug, '-', ' ') }}
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

### Loop position with `enumerate`

Use `enumerate(list)` to get the 0-based index alongside each item:

```
{% for e in enumerate(items) %}{{ e.index }}: {{ e.value }} {% endfor %}
```

Each element of the resulting list is a `Map` with keys `"index"` and `"value"`. When passing one to a fragment, the keys are promoted to the fragment's top-level context:

```
{% for e in enumerate(items) %}{{ Row e }}{% endfor %}
{# Inside Row: {{ index }} and {{ value.name }} — not e.index / e.value #}
```

## Whitespace control

By default every character in the template — including newlines around block tags — is emitted literally. Add a `-` on the *inside* of any tag delimiter to strip adjacent whitespace from the neighbouring text:

| Marker | Effect |
|--------|--------|
| `{{- expr }}` | strip whitespace **before** the output tag |
| `{{ expr -}}` | strip whitespace **after** the output tag |
| `{%- tag %}` | strip whitespace **before** the block tag |
| `{% tag -%}` | strip whitespace **after** the block tag |
| `{#- comment -#}` | strip whitespace on both sides of a comment |

Both sides can be combined on the same tag. "Whitespace" means any run of spaces, tabs, and newlines.

**Typical use — remove the newline that follows a block tag:**

```
{%- for item in items -%}
{{ item }},
{% endfor -%}
```

The `-%}` after `for` eats the newline that would otherwise appear before the first item; the `{%-` before `endfor` eats the newline after the last item's body.

**Strip whitespace around an inline output:**

```
Size: {{- len(items) -}} items
```

## Examples

```
cargo run --example report              # Employee report: DataSource on Rust structs
cargo run --example changelog           # Changelog: ctx! macro, string functions, custom functions
cargo run --example string_builtins     # upper/lower/trim/replace for slug and label generation
cargo run --example boolean_ops         # and/or operators for role/permission logic
cargo run --example enumerate           # enumerate() for numbered lists and position-aware rendering
cargo run --example whitespace_control  # {{-, -}}, {%-, -%} to trim whitespace around tags
cargo run --example arithmetic          # +, -, *, /, % operators and parenthesised grouping
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
  report.rs               — employee report using DataSource on Rust structs
  changelog.rs            — changelog generator using ctx! macro and custom functions
  string_builtins.rs      — upper/lower/trim/replace for slug and label normalisation
  boolean_ops.rs          — and/or operators for role/permission logic in fragments
  enumerate.rs            — enumerate() for numbered lists and position-aware rendering
  whitespace_control.rs   — {{-, -}}, {%-, -%} markers for compact output
  arithmetic.rs           — +, -, *, /, % operators and parenthesised grouping
```
