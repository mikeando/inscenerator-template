# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project context

`inscenerator-template` is part of the **inscenerator** suite. Related libraries:
- `inscenerator-xfs` — available on crates.io; typically checked out alongside this repo during development
- `inscenerator-entity` — available on crates.io; typically checked out alongside this repo during development
- `inscenerator-booker` — the primary consuming application (private)

This library is intended to be published to crates.io. Keep the public API stable and the `[dependencies]` section of `Cargo.toml` minimal.

## Commands

```bash
cargo build          # build the library
cargo test           # run all tests
cargo test <name>    # run a single test by name (substring match)
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

This is a pure Rust library (no external dependencies) implementing a text templating engine. The pipeline is:

**`lexer.rs`** → tokenizes source text into `Token` variants. Fragment calls (`{{ FragmentName expr }}`) are distinguished from plain output at lex time: if the first word starts with an ASCII uppercase letter and is followed by whitespace, it's a `FragmentCall`; otherwise it's `Output`.

**`expr.rs`** → defines the `Expr` AST (`Literal`, `Path`, `Not`, `BinOp`, `Call`) and the `parse_expr()` function. Expressions are parsed **at template parse time** (not lazily at render time) via a small internal lexer → recursive-descent parser. Grammar:
```
expr       := comparison
comparison := unary (('==' | '!=' | '<=' | '>=' | '<' | '>') unary)?
unary      := '!' unary | primary
primary    := literal | call | path
call       := ident '(' (expr (',' expr)*)? ')'
path       := ident ('.' ident)*
```

**`parser.rs`** → converts tokens into an AST (`Node` enum) stored in a `Template` struct. All `String` expression fields in `Node` are now `Expr` (parsed eagerly). `Template` also holds `functions: HashMap<String, Arc<dyn Function>>` pre-populated with built-ins (`len`, `starts_with`, `ends_with`, `contains`). Fragments are parsed via `add_fragment()`; custom functions are registered via `add_function()`.

**`renderer.rs`** → walks the AST, evaluates `Expr` via `eval(&Expr, ctx, template)`, and dispatches fragment calls. Key behaviors:
- `eval()` handles all `Expr` variants: literals, path lookups, negation, binary operators (`apply_binop()`), and function calls (looked up in `template.functions`).
- Comparison `==`/`!=`: accepts `Int`/`Float` (auto-coerce), `Bool`, `Str`, `Null`; errors on mixed non-numeric types. Ordering operators: numeric only.
- Conditionals **require** `Value::Bool` — non-bool values in `{% if %}` / `{% elif %}` are runtime errors.
- `{% for %}` loops use `LoopContext` to overlay the loop variable onto the parent context (parent scope remains accessible inside loops).
- Fragment contexts are strictly scoped: a fragment only sees the `Value::Map` passed to it, not the outer context.
- Recursion depth is capped at 20 to guard against infinite fragment recursion.

**`value.rs`** → defines `Value` (the universal value type), the `DataSource` trait, and the `Function` trait. Uses `Arc<str>` and `Arc<dyn DataSource>` to avoid lifetime parameters. The `Function` trait has a blanket impl for `Fn(&[Value]) -> Result<Value, String> + Send + Sync`. Two macros are defined here:
- `ctx!` — preferred macro, supports nested maps (`{ ... }`) and lists (`[ ... ]`) with `:` syntax
- `context!` — legacy flat macro using `=>` syntax (kept for backward compatibility)

**`integration.rs`** — end-to-end tests; compiled only under `#[cfg(test)]`.

## Key design rules

- **`DataSource` trait** is the integration point: implement `fn get(&self, key: &str) -> Value` on any type to use it as a template context directly — no serialization needed.
- **`Function` trait** (`value.rs`) is the function integration point: implement it directly or use a `Fn(&[Value]) -> Result<Value, String> + Send + Sync` closure. Register with `Template::add_function()`.
- Missing variables resolve to `Value::Null`; rendering `Null` produces an empty string. But accessing a variable that returns `Null` via a plain `{{ expr }}` output node is a runtime error (`"Variable not found: \`...\`"`).
- `Value::Map(Arc<dyn DataSource>)` is how nested objects and loop items carry their own type-safe data.
- Expressions are parsed at template parse time into `Expr` AST nodes, not lazily at render time. Invalid expressions are caught at `Template::parse()` time.
