/// Demonstrates subscript path access (`.[expr]`), null-safe operators (`.?`, `.?[`),
/// and null-coalescing (`??`).
///
/// Subscript access (`.[expr]`):
///   data.["key"]         — access a map with a string literal key
///   data.[var]           — access a map using a variable that holds the key name
///   obj.["a"].["b"].c    — mix subscript and dot steps freely
///
/// Null-safe access (`.?`, `.?[`):
///   a.?b                 — if `a` is null or `b` is missing, return null (no error)
///   a.?["key"]           — null-safe subscript: same but using a subscript key
///   a.?b.?c              — chain propagates null through the entire chain
///
/// Null-coalescing (`??`):
///   a ?? fallback        — if `a` is null, return `fallback`; otherwise return `a`
///   a.?b ?? "default"    — the idiomatic pattern for optional fields
///
/// Important: `??` catches Value::Null — not missing variables.
/// Use `.?`/`.?[` to produce null from an absent key, then `??` to provide a default.
use inscenerator_template::{Template, ctx, render};

fn main() {
    // ------------------------------------------------------------------ //
    // 1. Subscript access — string literal key
    //
    // Chapters keyed by slug, not by a simple integer index.
    // "010_intro" is not a valid identifier, so dot access (obj.010_intro)
    // would be a syntax error. Subscript access handles it cleanly.
    // ------------------------------------------------------------------ //
    let book = ctx! {
        "chapters": {
            "010_intro":    { "title": "Introduction",  "pages": 12 },
            "020_setup":    { "title": "Getting Started", "pages": 24 },
            "030_advanced": { "title": "Advanced Topics", "pages": 40 }
        }
    };

    let tmpl = Template::parse(
        r#"{{ chapters.["010_intro"].title }} ({{ chapters.["010_intro"].pages }} pages)"#,
    )
    .unwrap();
    println!("{}", render(&tmpl, book.as_ref()).unwrap());
    // → Introduction (12 pages)

    // ------------------------------------------------------------------ //
    // 2. Subscript access — dynamic key from a variable
    //
    // The key is held in a context variable. Useful when the key is computed
    // or passed in from outside the template.
    // ------------------------------------------------------------------ //
    let ctx = ctx! {
        "chapters": {
            "010_intro":  { "title": "Introduction" },
            "020_setup":  { "title": "Getting Started" }
        },
        "selected": "020_setup"
    };

    let tmpl = Template::parse("Selected: {{ chapters.[selected].title }}\n").unwrap();
    println!("{}", render(&tmpl, ctx.as_ref()).unwrap());
    // → Selected: Getting Started

    // ------------------------------------------------------------------ //
    // 3. Null-safe dot access — optional field that may be absent
    //
    // A book may or may not have a subtitle. Accessing book.subtitle without
    // null-safe access would error when the field is absent.
    // ------------------------------------------------------------------ //
    let with_subtitle = ctx! {
        "book": { "title": "Rust in Action", "subtitle": "Systems Programming" }
    };
    let without_subtitle = ctx! {
        "book": { "title": "The Rust Book" }
        // no "subtitle" key
    };

    let tmpl = Template::parse(r#"{{ book.title }}: {{ book.?subtitle ?? "" }}"#).unwrap();

    // When subtitle is present, it's returned:
    let out = render(&tmpl, with_subtitle.as_ref()).unwrap();
    println!("{}", out.trim());
    // → Rust in ActionSystems Programming

    // When subtitle is absent, .? returns null and ?? substitutes "":
    let out = render(&tmpl, without_subtitle.as_ref()).unwrap();
    println!("{}", out.trim());
    // → The Rust Book

    // ------------------------------------------------------------------ //
    // 4. Null-safe subscript — access a map by a non-identifier key safely
    // ------------------------------------------------------------------ //
    let project = ctx! {
        "chapters": {
            "010_intro": { "title": "Introduction" }
        }
    };

    // "020_setup" is absent — .?[ returns null, ?? gives the fallback.
    let tmpl = Template::parse(
        [
            r#"Intro:   {{ chapters.?["010_intro"].?title ?? "(missing)" }}"#,
            r#"Chapter: {{ chapters.?["020_setup"].?title ?? "(missing)" }}"#,
        ]
        .join("\n")
        .as_str(),
    )
    .unwrap();
    println!("{}", render(&tmpl, project.as_ref()).unwrap());
    // → Intro:   Introduction
    // → Chapter: (missing)

    // ------------------------------------------------------------------ //
    // 5. Chained null-safe access
    //
    // Each .? propagates null through the chain, so a single absent key
    // anywhere in the chain silently produces null for the whole expression.
    // ------------------------------------------------------------------ //
    let deep = ctx! {
        "config": {
            "database": { "host": "localhost", "port": 5432 }
        }
    };

    // Full path present:
    let tmpl = Template::parse(r#"Host: {{ config.?database.?host ?? "(unset)" }}"#).unwrap();
    println!("{}", render(&tmpl, deep.as_ref()).unwrap());
    // → Host: localhost

    // Absent at second level — the whole chain returns null:
    let shallow = ctx! { "config": {} };
    println!("{}", render(&tmpl, shallow.as_ref()).unwrap());
    // → Host: (unset)

    // Absent at first level:
    let _empty = ctx! {};
    // Note: "config" is not found via strict lookup, so we need .? from the root.
    // Use a wrapper:
    let tmpl2 =
        Template::parse(r#"Host: {{ root.?config.?database.?host ?? "(unset)" }}"#).unwrap();
    let wrapped = ctx! { "root": {} };
    println!("{}", render(&tmpl2, wrapped.as_ref()).unwrap());
    // → Host: (unset)

    // ------------------------------------------------------------------ //
    // 6. Real-world pattern — optional metadata in a document template
    //
    // Documents have required fields (always present) and optional metadata
    // that may or may not be populated.
    // ------------------------------------------------------------------ //
    let doc_full = ctx! {
        "meta": {
            "title":   "Chapter One",
            "author":  "Alice",
            "version": "2.1"
        }
    };
    let doc_partial = ctx! {
        "meta": {
            "title": "Chapter Two"
            // author and version absent
        }
    };

    let header = Template::parse(
        [
            r#"Title:   {{ meta.title }}"#,
            r#"Author:  {{ meta.?author ?? "Unknown" }}"#,
            r#"Version: {{ meta.?version ?? "draft" }}"#,
        ]
        .join("\n")
        .as_str(),
    )
    .unwrap();

    println!("{}", render(&header, doc_full.as_ref()).unwrap());
    // → Title:   Chapter One
    // → Author:  Alice
    // → Version: 2.1

    println!("{}", render(&header, doc_partial.as_ref()).unwrap());
    // → Title:   Chapter Two
    // → Author:  Unknown
    // → Version: draft

    // ------------------------------------------------------------------ //
    // 7. Root-level subscript access
    //
    // When you want to index the root context directly, use a leading dot.
    // .[expr] subscripts the root context — useful when the key is a
    // non-identifier (like "010_intro") or comes from a variable.
    // ------------------------------------------------------------------ //
    let ctx = ctx! {
        "010_intro": { "title": "Introduction",  "pages": 12 },
        "020_setup": { "title": "Getting Started", "pages": 24 },
        "selected": "020_setup"
    };

    // Literal key on root:
    let tmpl = Template::parse(r#"{{ .["010_intro"].title }}"#).unwrap();
    println!("{}", render(&tmpl, ctx.as_ref()).unwrap().trim());
    // → Introduction

    // Dynamic key on root:
    let tmpl = Template::parse(r#"{{ .[selected].title }}"#).unwrap();
    println!("{}", render(&tmpl, ctx.as_ref()).unwrap().trim());
    // → Getting Started

    // Null-safe root lookup:
    let tmpl = Template::parse(r#"{{ .?missing_key ?? "(not set)" }}"#).unwrap();
    println!("{}", render(&tmpl, ctx.as_ref()).unwrap().trim());
    // → (not set)
}
