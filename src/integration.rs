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
    fn get(&self, key: &str) -> Option<Value> {
        match key {
            "name" => Some(Value::from(self.name)),
            "age" => Some(Value::Int(self.age)),
            "active" => Some(Value::Bool(self.active)),
            _ => None,
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
        fn get(&self, key: &str) -> Option<Value> {
            match key {
                "label" => Some(Value::from(self.label)),
                _ => None,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::DataSource(Arc::new(Item { label: "foo" })),
        Value::DataSource(Arc::new(Item { label: "bar" })),
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
        fn get(&self, key: &str) -> Option<Value> {
            match key {
                "label" => Some(Value::from(self.label)),
                _ => None,
            }
        }
    }

    let items = Value::List(Arc::new(vec![
        Value::DataSource(Arc::new(Item { label: "foo" })),
        Value::DataSource(Arc::new(Item { label: "bar" })),
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
        fn get(&self, _: &str) -> Option<Value> {
            None
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
        fn get(&self, key: &str) -> Option<Value> {
            match key {
                "city" => Some(Value::from(self.city)),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    struct Person {
        address: Address,
    }
    impl DataSource for Person {
        fn get(&self, key: &str) -> Option<Value> {
            match key {
                "address" => Some(Value::DataSource(Arc::new(Address {
                    city: self.address.city,
                }))),
                _ => None,
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
    tmpl.add_fragment(
        "CurrencySymbol",
        r#"{% if is_usd %}${% else %}€{% endif %}"#,
    )
    .unwrap();

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
    // With the Expr AST, "a b c" is rejected at parse time.
    let err = Template::parse("{{ a b c }}").unwrap_err();
    assert!(
        err.contains("Invalid expression") || err.contains("Unexpected token"),
        "error was: {:?}",
        err
    );
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
        fn get(&self, key: &str) -> Option<Value> {
            if key == "self" {
                Some(Value::DataSource(Arc::new(RecursiveDS)))
            } else {
                None
            }
        }
    }

    let err = render(&tmpl, &RecursiveDS).unwrap_err();
    assert!(
        err.contains("Maximum recursion depth exceeded (possible infinite fragment recursion)"),
        "error was: {:?}",
        err
    );
}

#[test]
fn test_non_bool_conditional_error() {
    let tmpl = Template::parse("{% if val %}YES{% endif %}").unwrap();
    let ctx = ctx! { "val": "non-bool" };
    // User wants non-bools in conditionals to be an error
    let err = render(&tmpl, ctx.as_ref()).unwrap_err();
    assert!(
        err.contains("Conditional expression `val` must evaluate to a Bool, got `non-bool`"),
        "error was: {:?}",
        err
    );
}

// --- Comparison operators ---

#[test]
fn test_comparison_eq_str() {
    let src = "{% if name == 'Alice' %}match{% else %}no{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "name": "Alice" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "match");
    let ctx2 = ctx! { "name": "Bob" };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_comparison_ne_int() {
    let src = "{% if count != 0 %}non-zero{% else %}zero{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "count": 5 };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "non-zero");
    let ctx2 = ctx! { "count": 0 };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "zero");
}

#[test]
fn test_comparison_lt_gt() {
    let tmpl = Template::parse("{% if age < 18 %}minor{% else %}adult{% endif %}").unwrap();
    let young = ctx! { "age": 15 };
    assert_eq!(render(&tmpl, young.as_ref()).unwrap(), "minor");
    let old = ctx! { "age": 21 };
    assert_eq!(render(&tmpl, old.as_ref()).unwrap(), "adult");
}

#[test]
fn test_comparison_le_ge() {
    let tmpl = Template::parse("{% if score >= 90 %}A{% else %}B{% endif %}").unwrap();
    let high = ctx! { "score": 90 };
    assert_eq!(render(&tmpl, high.as_ref()).unwrap(), "A");
    let low = ctx! { "score": 89 };
    assert_eq!(render(&tmpl, low.as_ref()).unwrap(), "B");
}

#[test]
fn test_comparison_int_float_coerce() {
    let tmpl = Template::parse("{% if ratio > 1 %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "ratio": 1.5f64 };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_comparison_mixed_type_error() {
    let tmpl = Template::parse("{% if name == 42 %}yes{% endif %}").unwrap();
    let ctx = ctx! { "name": "Alice" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_comparison_output_value() {
    // Comparison result used directly in output
    let tmpl = Template::parse("{{ age >= 18 }}").unwrap();
    let ctx = ctx! { "age": 21 };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "true");
}

// --- Built-in functions ---

#[test]
fn test_builtin_len_str() {
    let tmpl = Template::parse("{{ len(name) }}").unwrap();
    let ctx = ctx! { "name": "hello" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "5");
}

#[test]
fn test_builtin_len_str_unicode() {
    // len() counts characters, not bytes
    let tmpl = Template::parse("{{ len(name) }}").unwrap();
    let ctx = ctx! { "name": "héllo" }; // 5 chars, 6 bytes
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "5");
}

#[test]
fn test_builtin_len_list() {
    let tmpl = Template::parse("{{ len(items) }}").unwrap();
    let ctx = ctx! { "items": ["a", "b", "c"] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "3");
}

#[test]
fn test_builtin_len_wrong_type() {
    let tmpl = Template::parse("{{ len(count) }}").unwrap();
    let ctx = ctx! { "count": 42 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_len_wrong_arg_count() {
    let tmpl = Template::parse("{{ len(a, b) }}").unwrap();
    let ctx = ctx! { "a": "foo", "b": "bar" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_ends_with() {
    let src = "{% if ends_with(name, '.rs') %}yes{% else %}no{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "name": "main.rs" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
    let ctx2 = ctx! { "name": "main.py" };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_builtin_ends_with_wrong_type() {
    let tmpl = Template::parse("{{ ends_with(count, '.rs') }}").unwrap();
    let ctx = ctx! { "count": 42 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_ends_with_wrong_arg_count() {
    let tmpl = Template::parse("{{ ends_with(name) }}").unwrap();
    let ctx = ctx! { "name": "main.rs" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_starts_with() {
    let src = "{% if starts_with(greeting, 'Hello') %}yes{% else %}no{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "greeting": "Hello, World" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
    let ctx2 = ctx! { "greeting": "Hi, World" };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_builtin_starts_with_wrong_type() {
    let tmpl = Template::parse("{{ starts_with(count, 'x') }}").unwrap();
    let ctx = ctx! { "count": 1 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_contains() {
    let src = "{% if contains(text, 'world') %}yes{% else %}no{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "text": "hello world" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
    let ctx2 = ctx! { "text": "hello there" };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_builtin_contains_wrong_type() {
    let tmpl = Template::parse("{{ contains(count, '1') }}").unwrap();
    let ctx = ctx! { "count": 1 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_contains_wrong_arg_count() {
    let tmpl = Template::parse("{{ contains(text) }}").unwrap();
    let ctx = ctx! { "text": "hello" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

// --- Custom functions ---

#[test]
fn test_custom_function() {
    let mut tmpl = Template::parse("{{ greet(name) }}").unwrap();
    tmpl.add_function("greet", |args: &[crate::Value]| match args {
        [crate::Value::Str(s)] => Ok(crate::Value::from(format!("Hello, {}!", s).as_str())),
        _ => Err("greet() expects one Str".to_string()),
    });
    let ctx = ctx! { "name": "World" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Hello, World!");
}

#[test]
fn test_custom_function_shadows_builtin() {
    let mut tmpl = Template::parse("{{ len(name) }}").unwrap();
    tmpl.add_function("len", |_args: &[crate::Value]| Ok(crate::Value::Int(42)));
    let ctx = ctx! { "name": "anything" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "42");
}

#[test]
fn test_unknown_function_error() {
    let tmpl = Template::parse("{{ no_such_fn(name) }}").unwrap();
    let ctx = ctx! { "name": "World" };
    let err = render(&tmpl, ctx.as_ref()).unwrap_err();
    assert!(err.contains("Unknown function"), "error was: {:?}", err);
}

#[test]
fn test_function_in_conditional() {
    // ends_with used directly as a conditional
    let src = "{% if ends_with(file, '.rs') %}Rust{% else %}Other{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "file": "lib.rs" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "Rust");
}

#[test]
fn test_function_with_dotted_path_arg() {
    let src = "{% if starts_with(user.name, 'A') %}yes{% else %}no{% endif %}";
    let tmpl = Template::parse(src).unwrap();
    let ctx = ctx! { "user": { "name": "Alice" } };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

// --- String built-ins: upper, lower, trim, replace ---

#[test]
fn test_builtin_upper() {
    let tmpl = Template::parse("{{ upper(name) }}").unwrap();
    let ctx = ctx! { "name": "hello" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "HELLO");
}

#[test]
fn test_builtin_upper_unicode() {
    let tmpl = Template::parse("{{ upper(word) }}").unwrap();
    let ctx = ctx! { "word": "café" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "CAFÉ");
}

#[test]
fn test_builtin_upper_wrong_type() {
    let tmpl = Template::parse("{{ upper(count) }}").unwrap();
    let ctx = ctx! { "count": 42 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_upper_wrong_arg_count() {
    let tmpl = Template::parse("{{ upper(a, b) }}").unwrap();
    let ctx = ctx! { "a": "x", "b": "y" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_lower() {
    let tmpl = Template::parse("{{ lower(name) }}").unwrap();
    let ctx = ctx! { "name": "HELLO" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "hello");
}

#[test]
fn test_builtin_lower_wrong_type() {
    let tmpl = Template::parse("{{ lower(count) }}").unwrap();
    let ctx = ctx! { "count": 1 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_trim() {
    let tmpl = Template::parse("'{{ trim(text) }}'").unwrap();
    let ctx = ctx! { "text": "  hello  " };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "'hello'");
}

#[test]
fn test_builtin_trim_no_whitespace() {
    let tmpl = Template::parse("{{ trim(text) }}").unwrap();
    let ctx = ctx! { "text": "hello" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "hello");
}

#[test]
fn test_builtin_trim_wrong_type() {
    let tmpl = Template::parse("{{ trim(count) }}").unwrap();
    let ctx = ctx! { "count": 3 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_replace() {
    let tmpl = Template::parse("{{ replace(text, '-', ' ') }}").unwrap();
    let ctx = ctx! { "text": "hello-world" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "hello world");
}

#[test]
fn test_builtin_replace_multiple_occurrences() {
    let tmpl = Template::parse("{{ replace(text, 'o', '0') }}").unwrap();
    let ctx = ctx! { "text": "foo bar boo" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "f00 bar b00");
}

#[test]
fn test_builtin_replace_no_match() {
    let tmpl = Template::parse("{{ replace(text, 'x', 'y') }}").unwrap();
    let ctx = ctx! { "text": "hello" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "hello");
}

#[test]
fn test_builtin_replace_wrong_type() {
    let tmpl = Template::parse("{{ replace(count, '1', '2') }}").unwrap();
    let ctx = ctx! { "count": 1 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_builtin_replace_wrong_arg_count() {
    let tmpl = Template::parse("{{ replace(text, '-') }}").unwrap();
    let ctx = ctx! { "text": "a-b" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

// --- Boolean and / or operators ---

#[test]
fn test_and_both_true() {
    let tmpl = Template::parse("{% if a and b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": true, "b": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_and_one_false() {
    let tmpl = Template::parse("{% if a and b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": true, "b": false };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "no");
}

#[test]
fn test_and_both_false() {
    let tmpl = Template::parse("{% if a and b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": false, "b": false };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "no");
}

#[test]
fn test_or_both_false() {
    let tmpl = Template::parse("{% if a or b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": false, "b": false };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "no");
}

#[test]
fn test_or_one_true() {
    let tmpl = Template::parse("{% if a or b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": false, "b": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_symbolic_and() {
    let tmpl = Template::parse("{% if a && b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": true, "b": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_symbolic_or() {
    let tmpl = Template::parse("{% if a || b %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": false, "b": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
}

#[test]
fn test_and_with_comparisons() {
    // Compound condition: role == "admin" and active
    let tmpl = Template::parse(
        "{% if role == 'admin' and active %}allowed{% else %}denied{% endif %}",
    )
    .unwrap();
    let ctx = ctx! { "role": "admin", "active": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "allowed");
    let ctx2 = ctx! { "role": "admin", "active": false };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "denied");
    let ctx3 = ctx! { "role": "guest", "active": true };
    assert_eq!(render(&tmpl, ctx3.as_ref()).unwrap(), "denied");
}

#[test]
fn test_or_with_comparisons() {
    let tmpl = Template::parse(
        "{% if role == 'admin' or role == 'editor' %}yes{% else %}no{% endif %}",
    )
    .unwrap();
    let ctx = ctx! { "role": "editor" };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
    let ctx2 = ctx! { "role": "viewer" };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_chained_and() {
    let tmpl =
        Template::parse("{% if a and b and c %}yes{% else %}no{% endif %}").unwrap();
    let ctx = ctx! { "a": true, "b": true, "c": true };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "yes");
    let ctx2 = ctx! { "a": true, "b": false, "c": true };
    assert_eq!(render(&tmpl, ctx2.as_ref()).unwrap(), "no");
}

#[test]
fn test_and_non_bool_error() {
    let tmpl = Template::parse("{% if a and b %}yes{% endif %}").unwrap();
    let ctx = ctx! { "a": true, "b": 42 };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_and_keyword_cannot_be_variable() {
    assert!(Template::parse("{{ and }}").is_err());
}

#[test]
fn test_or_keyword_cannot_be_variable() {
    assert!(Template::parse("{{ or }}").is_err());
}

// --- enumerate() built-in ---

#[test]
fn test_enumerate_indices() {
    // index starts at 0 and increments
    let tmpl = Template::parse("{% for e in enumerate(items) %}{{ e.index }}{% endfor %}").unwrap();
    let ctx = ctx! { "items": ["a", "b", "c"] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "012");
}

#[test]
fn test_enumerate_values() {
    // original values are accessible via e.value
    let tmpl =
        Template::parse("{% for e in enumerate(items) %}{{ e.value }}{% endfor %}").unwrap();
    let ctx = ctx! { "items": ["x", "y", "z"] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "xyz");
}

#[test]
fn test_enumerate_index_and_value() {
    let tmpl =
        Template::parse("{% for e in enumerate(items) %}{{ e.index }}:{{ e.value }} {% endfor %}")
            .unwrap();
    let ctx = ctx! { "items": ["a", "b", "c"] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "0:a 1:b 2:c ");
}

#[test]
fn test_enumerate_nested_map_value() {
    // Works with list-of-maps: e.value.name
    let tmpl =
        Template::parse("{% for e in enumerate(items) %}{{ e.index }}.{{ e.value.name }} {% endfor %}")
            .unwrap();
    let ctx = ctx! {
        "items": [
            { "name": "Alice" },
            { "name": "Bob" }
        ]
    };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "0.Alice 1.Bob ");
}

#[test]
fn test_enumerate_last_item_check() {
    // Common use-case: suppress trailing comma on last item using index comparison
    let tmpl = Template::parse(
        "{% for e in enumerate(items) %}{{ e.value }}{% if e.index < 2 %},{% endif %}{% endfor %}",
    )
    .unwrap();
    let ctx = ctx! { "items": ["a", "b", "c"] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "a,b,c");
}

#[test]
fn test_enumerate_empty_list() {
    let tmpl =
        Template::parse("{% for e in enumerate(items) %}x{% endfor %}nothing").unwrap();
    let ctx = ctx! { "items": [] };
    assert_eq!(render(&tmpl, ctx.as_ref()).unwrap(), "nothing");
}

#[test]
fn test_enumerate_wrong_type() {
    let tmpl = Template::parse("{{ enumerate(name) }}").unwrap();
    let ctx = ctx! { "name": "Alice" };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}

#[test]
fn test_enumerate_wrong_arg_count() {
    let tmpl = Template::parse("{{ enumerate(a, b) }}").unwrap();
    let ctx = ctx! { "a": ["x"], "b": ["y"] };
    assert!(render(&tmpl, ctx.as_ref()).is_err());
}
