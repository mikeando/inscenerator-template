use crate::{DataSource, Template, Value, ctx, render};
use std::sync::Arc;

// --- A real struct implementing DataSource ---

#[derive(Debug)]
struct User {
    name: &'static str,
    age: i64,
    active: bool,
}

impl DataSource for User {
    fn get(&self, key: &str) -> Value {
        match key {
            "name" => Value::from(self.name),
            "age" => Value::Int(self.age),
            "active" => Value::Bool(self.active),
            _ => Value::Null,
        }
    }
}

#[test]
fn test_simple_output() {
    let tmpl = Template::parse("Hello, {{ name }}!").unwrap();
    let ctx = ctx! { "name": "World" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Hello, World!");
}

#[test]
fn test_struct_datasource() {
    let tmpl = Template::parse("{{ name }} is {{ age }} years old.").unwrap();
    let user = User {
        name: "Alice",
        age: 30,
        active: true,
    };
    assert_eq!(render(&tmpl, &user).unwrap(), "Alice is 30 years old.");
}

#[test]
fn test_if_else() {
    let src = "{% if active %}on{% else %}off{% endif %}";
    let tmpl = Template::parse(src).unwrap();

    let on = User {
        name: "Alice",
        age: 30,
        active: true,
    };
    let off = User {
        name: "Bob",
        age: 25,
        active: false,
    };

    assert_eq!(render(&tmpl, &on).unwrap(), "on");
    assert_eq!(render(&tmpl, &off).unwrap(), "off");
}

#[test]
fn test_elif() {
    let src = "{% if a %}A{% elif b %}B{% else %}C{% endif %}";
    let tmpl = Template::parse(src).unwrap();

    let ctx_a = ctx! { "a": true,  "b": false };
    let ctx_b = ctx! { "a": false, "b": true };
    let ctx_c = ctx! { "a": false, "b": false };

    assert_eq!(render(&tmpl, ctx_a.as_ref()).unwrap(), "A");
    assert_eq!(render(&tmpl, ctx_b.as_ref()).unwrap(), "B");
    assert_eq!(render(&tmpl, ctx_c.as_ref()).unwrap(), "C");
}

#[test]
fn test_for_loop() {
    let src = "{% for item in items %}[{{ item }}]{% endfor %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! {
        "items": ["a", "b", "c"]
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "[a][b][c]");
}

#[test]
fn test_nested_map_in_loop() {
    // Each item in the list is itself a Map (DataSource)
    #[derive(Debug)]
    struct Item {
        label: &'static str,
    }
    impl DataSource for Item {
        fn get(&self, key: &str) -> Value {
            match key {
                "label" => Value::from(self.label),
                _ => Value::Null,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::Map(Arc::new(Item { label: "foo" })),
        Value::Map(Arc::new(Item { label: "bar" })),
    ]));

    let src = "{% for it in items %}{{ it.label }} {% endfor %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "items": items };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "foo bar ");
}

#[test]
fn test_fragment_call_with_map_context() {
    // A fragment is called with {{ FragmentName expr }} where expr resolves to a Map.
    // The fragment's variables resolve against that map, not the outer context.
    let mut tmpl = Template::parse("{{ ProductView product }}").unwrap();
    tmpl.add_fragment("ProductView", "{{ name }}: ${{ price }}")
        .unwrap();

    let ctx = ctx! {
        "product": { "name": "Widget", "price": 9 }
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Widget: $9");
}

#[test]
fn test_fragment_call_in_loop() {
    // The canonical use case: each loop item is passed to a fragment as its context.
    #[derive(Debug)]
    struct Item {
        label: &'static str,
    }
    impl DataSource for Item {
        fn get(&self, key: &str) -> Value {
            match key {
                "label" => Value::from(self.label),
                _ => Value::Null,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::Map(Arc::new(Item { label: "foo" })),
        Value::Map(Arc::new(Item { label: "bar" })),
    ]));

    let mut tmpl = Template::parse("{% for it in items %}{{ Row it }}{% endfor %}").unwrap();
    tmpl.add_fragment("Row", "[{{ label }}]").unwrap();

    let ctx = ctx! { "items": items };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "[foo][bar]");
}

#[test]
fn test_fragment_unknown_errors() {
    let _tmpl = Template::parse("{{ Missing thing }}").unwrap();
    let _ctx = ctx! {
        "thing": {}
    };
    // Can't easily use Value::Map(context!{}) directly here, so test via a known-missing name
    let tmpl2 = Template::parse("{{ Ghost x }}").unwrap();
    #[derive(Debug)]
    struct _Dummy;
    impl DataSource for _Dummy {
        fn get(&self, _: &str) -> Value {
            Value::Null
        }
    }
    // context expr resolving to Null should return an error
    let ctx2 = ctx! {};
    assert!(render(&tmpl2, ctx2.as_ref()).is_err());
}

#[test]
fn test_comment_ignored() {
    let tmpl = Template::parse("a{# this is a comment #}b").unwrap();
    let ctx = ctx! {};
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "ab");
}

#[test]
fn test_negation() {
    let src = "{% if !flag %}yes{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "flag": false };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_dotted_path() {
    #[derive(Debug)]
    struct Address {
        city: &'static str,
    }
    impl DataSource for Address {
        fn get(&self, key: &str) -> Value {
            match key {
                "city" => Value::from(self.city),
                _ => Value::Null,
            }
        }
    }

    #[derive(Debug)]
    struct Person {
        address: Address,
    }
    impl DataSource for Person {
        fn get(&self, key: &str) -> Value {
            match key {
                "address" => Value::Map(Arc::new(Address {
                    city: self.address.city,
                })),
                _ => Value::Null,
            }
        }
    }

    let tmpl = Template::parse("{{ address.city }}").unwrap();
    let p = Person {
        address: Address { city: "London" },
    };
    assert_eq!(render(&tmpl, &p).unwrap(), "London");
}

#[test]
fn test_complex_nesting() {
    let src = r#"
        {% for category in categories %}
            Category: {{ category.name }}
            {% for product in category.products %}
                - {{ product.name }} ({{ CurrencySymbol product }})
            {% endfor %}
        {% endfor %}
    "#;
    let mut tmpl = Template::parse(src).unwrap();
    tmpl.add_fragment("CurrencySymbol", r#"{% if is_usd %}${% else %}€{% endif %}"#).unwrap();

    let ctx = ctx! {
        "categories": [
            {
                "name": "Electronics",
                "products": [
                    { "name": "Phone", "is_usd": true },
                    { "name": "Laptop", "is_usd": false }
                ]
            }
        ]
    };

    let output = render(&tmpl, ctx.as_ref()).unwrap();
    assert!(output.contains("Category: Electronics"));
    assert!(output.contains("- Phone ($)"));
    assert!(output.contains("- Laptop (€)"));
}

#[test]
fn test_utf8_integration() {
    let src = "Hållø, {{ üsër }}! {% if håppÿ %}😊{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! {
        "üsër": "Wørld",
        "håppÿ": true
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Hållø, Wørld! 😊");
}

#[test]
fn test_missing_variable_errors() {
    let tmpl = Template::parse("Before{{ missing }}After").unwrap();
    let ctx = ctx! {};
    // Exposing bug: missing variables should be an error
    let err = render(&tmpl, ctx.as_ref()).unwrap_err();
    assert_eq!(err, "Variable not found: `missing` ");
}

#[test]
fn test_whitespace_preservation() {
    let src = "  {% if true %}  A  {% endif %}  ";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! {};
    // current implementation preserves everything outside tags
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "    A    ");
}

#[test]
fn test_invalid_expression_errors() {
    // Current eval() doesn't support errors. This should be an evaluation error.
    let tmpl = Template::parse("{{ a b c }}").unwrap();
    let ctx = ctx! {};
    let err = render(&tmpl, ctx.as_ref()).unwrap_err();
    assert!(err.contains("Invalid expression") || err.contains("Variable not found"));
}

#[test]
fn test_utf8_variable_names() {
    let tmpl = Template::parse("{{ nåmë }}").unwrap();
    let ctx = ctx! { "nåmë": "Ålïcë" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Ålïcë");
}

#[test]
fn test_deep_fragment_recursion() {
    let mut tmpl = Template::parse("{{ Recurse self }}").unwrap();
    tmpl.add_fragment("Recurse", "{{ Recurse self }}").unwrap();

    #[derive(Debug)]
    struct RecursiveDS;
    impl DataSource for RecursiveDS {
        fn get(&self, key: &str) -> Value {
            if key == "self" {
                Value::Map(Arc::new(RecursiveDS))
            } else {
                Value::Null
            }
        }
    }

    // This WILL stack overflow, exposing the lack of recursion limits.
    render(&tmpl, &RecursiveDS).unwrap();
}

#[test]
fn test_non_bool_conditional_error() {
    let tmpl = Template::parse("{% if val %}YES{% endif %}").unwrap();
    let ctx = ctx! { "val": "non-bool" };
    // User wants non-bools in conditionals to be an error
    let err = render(&tmpl, ctx.as_ref()).unwrap_err();
    assert!(err.contains("Conditional must be a boolean"));
}
