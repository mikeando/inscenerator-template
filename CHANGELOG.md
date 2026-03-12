# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-03-12

### Added
- Expression-based indexing: `a.[expr]` (error if key missing) and `a.?[expr]` (null if key missing)
- Optional chaining: `a.?b` evaluates to null if `a` is null or `b` is not a key; `a.b` remains an error in those cases
- Leading-dot path syntax: `.a` is equivalent to `a`; `.?a` resolves to null rather than error if `a` is missing from root
- Null-coalescing operator `??`: `a ?? b` returns `a` if non-null, otherwise `b`

## [0.1.1] - 2026-03-01

### Added
- Arithmetic operators: `+`, `-`, `*`, `/`, `%` and parenthesised grouping
- Whitespace control via `{{-`/`-}}` (output tags) and `{%-`/`-%}` (block tags)
- `enumerate()` built-in function for loop indexing
- Boolean `and`/`or` operators in expressions
- String built-in functions: `upper`, `lower`, `trim`, `replace`

## [0.1.0] - 2026-02-27

### Added
- Core templating engine: `{{ expr }}` output, `{% if %}`/`{% elif %}`/`{% else %}`, `{% for %}`, and `{% fragment %}` nodes
- Expression language with literals, path lookups, `!` negation, and comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- `DataSource` trait for type-safe context integration without serialization
- `Function` trait with blanket impl for `Fn(&[Value]) -> Result<Value, String> + Send + Sync` closures; registered via `Template::add_function()`
- Built-in functions: `len`, `starts_with`, `ends_with`, `contains`
- `ctx!` macro for nested context construction; `context!` legacy flat macro
- Expressions parsed eagerly at template-parse-time (errors caught at `Template::parse()`)
- Fragment recursion depth capped at 20

[Unreleased]: https://github.com/mikeando/inscenerator-template/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/mikeando/inscenerator-template/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mikeando/inscenerator-template/releases/tag/v0.1.0
