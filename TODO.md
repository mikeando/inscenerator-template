# TODO: Planned Features

High-impact improvements that are straightforward to implement. Each item gets its own commit, with tests and an example.

---

## 1. More string built-in functions

**Why:** Templates frequently need to transform strings — capitalise headings, trim whitespace, do simple substitutions. Right now the only escape hatch is registering custom functions in the host application, which is verbose for obvious utilities.

**Functions to add** (all pure, no architecture change — just new closures in `make_builtins()`):

| Function | Signature | Description |
|----------|-----------|-------------|
| `upper(s)` | `Str → Str` | ASCII uppercase |
| `lower(s)` | `Str → Str` | ASCII lowercase |
| `trim(s)` | `Str → Str` | Strip leading/trailing whitespace |
| `replace(s, from, to)` | `Str, Str, Str → Str` | Replace all occurrences of `from` with `to` |

Usage in a template:
```
{{ upper(title) }}
{{ trim(description) }}
{{ replace(slug, "-", " ") }}
```

---

## 2. Boolean `and` / `or` operators

**Why:** The most commonly reported friction point. Without boolean operators, multi-condition checks require nesting:

```
{# Current workaround — ugly #}
{% if is_admin %}{% if is_active %}...{% endif %}{% endif %}

{# What users expect #}
{% if is_admin and is_active %}...{% endif %}
{% if role == "editor" or role == "admin" %}...{% endif %}
```

**Implementation plan:**
- Add `And` and `Or` variants to `BinOp` in `expr.rs`
- Add `&&` / `||` tokens (and keyword aliases `and` / `or`) in the expr lexer
- Add a new `logical` precedence tier in the parser, lower than `comparison` (so `a == b and c == d` works intuitively)
- Evaluate in `apply_binop()` in `renderer.rs` — both operands must be `Bool`

Grammar after change:
```
expr       := logical
logical    := comparison (('and' | 'or' | '&&' | '||') comparison)*
comparison := unary (('==' | '!=' | '<=' | '>=' | '<' | '>') unary)?
unary      := '!' unary | primary
```

---

## 3. `enumerate(list)` built-in function

**Why:** Numbered lists, comma-separated output, alternating row styles, "last item" checks — all require knowing the current loop index. Rather than adding magic `loop.*` variables (which require renderer changes), an `enumerate()` function follows the existing function pattern and composes cleanly with the existing `{% for %}` loop.

**Behaviour:**
```
enumerate(list)  →  List of Maps, each with keys "index" (0-based Int) and "value" (original item)
```

Example template:
```
{% for entry in enumerate(items) %}
{{ entry.index }}. {{ entry.value.name }}{% if entry.index < len(items) - 1 %},{% endif %}
{% endfor %}
```

**Implementation plan:**
- Add `enumerate` closure to `make_builtins()` in `parser.rs`
- Each output element is a `Value::Map` backed by an `Arc<dyn DataSource>` struct holding `index: i64` and `value: Value`
- No changes to the parser, lexer, or renderer — pure built-in function

---

## 4. Whitespace control (`{%-` / `-%}`)

**Why:** Template blocks (`{% if %}`, `{% for %}`, `{% endif %}` etc.) emit the surrounding whitespace/newlines verbatim. For HTML this is usually harmless, but for plain-text, CSV, or structured-format output the extra blank lines are unwanted noise.

**Syntax:** A `-` immediately inside the opening or closing tag delimiter strips the whitespace adjacent to that delimiter:
- `{%-` strips whitespace/newlines *before* the tag
- `-%}` strips whitespace/newlines *after* the tag

**Implementation plan:**
- Extend the lexer: when `{%` is followed by optional whitespace then `-`, set a `trim_before` flag on the token; when `-` precedes `%}`, set `trim_after`
- Extend the `Token` variants (or add flags to existing control tokens) to carry these flags
- In the renderer, strip trailing whitespace from the last rendered text node when `trim_before` is set, and skip leading whitespace on the next text node when `trim_after` is set

Example:
```
Name: Alice
{%- if show_role %}
Role: Admin
{%- endif %}
Email: alice@example.com
```
Renders as:
```
Name: Alice
Role: Admin
Email: alice@example.com
```
instead of the current:
```
Name: Alice

Role: Admin

Email: alice@example.com
```
