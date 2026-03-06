/// Demonstrates arithmetic operators: `+`, `-`, `*`, `/`, `%`.
///
/// Operator precedence (high to low):
///   unary `-`   (negation)
///   `*`  `/`  `%`
///   `+`  `-`
///   `==`  `!=`  `<`  `>`  `<=`  `>=`
///   `and` / `&&`
///   `or`  / `||`
///
/// Parentheses `( expr )` can be used to override precedence.
///
/// Type rules:
///   Int OP Int   → Int   (/ truncates; % integer-only)
///   Float OP Float → Float
///   Int OP Float or Float OP Int → Float
///   Str + Str    → Str   (concatenation; only `+` supports strings)
use inscenerator_template::{Template, ctx, render};

fn main() {
    // ------------------------------------------------------------------ //
    // 1. Shopping cart — unit price × quantity, running total
    // ------------------------------------------------------------------ //
    let cart = ctx! {
        "items": [
            { "name": "Widget",  "price": 9,  "qty": 3 },
            { "name": "Gadget",  "price": 14, "qty": 1 },
            { "name": "Doohickey","price": 5, "qty": 5 }
        ],
        "tax_rate": 8
    };

    let receipt = Template::parse(concat!(
        "Receipt\n",
        "-------\n",
        "{% for item in items -%}\n",
        "{{ item.name }}: {{ item.price }} × {{ item.qty }} = {{ item.price * item.qty }}\n",
        "{% endfor -%}\n",
        "Tax ({{ tax_rate }}%): calculated at checkout\n",
    ))
    .unwrap();

    println!("{}", render(&receipt, cart.as_ref()).unwrap());

    // ------------------------------------------------------------------ //
    // 2. Integer division and modulo — pagination
    // ------------------------------------------------------------------ //
    let page = ctx! {
        "total":    47,
        "per_page": 10
    };

    // total / per_page truncates toward zero (integer ÷ integer → integer)
    let pager = Template::parse(concat!(
        "{{ total }} items, ",
        "{{ total / per_page }} full pages, ",
        "{{ total % per_page }} remaining\n",
    ))
    .unwrap();

    println!("{}", render(&pager, page.as_ref()).unwrap());
    // → 47 items, 4 full pages, 7 remaining

    // ------------------------------------------------------------------ //
    // 3. Float arithmetic — discount price
    // ------------------------------------------------------------------ //
    let product = ctx! {
        "price":    29,
        "discount": 0.15   // 15 %
    };

    // Multiplying Int × Float yields Float
    let discounted =
        Template::parse("After {{ discount * 100 }}% off: {{ price - price * discount }}\n")
            .unwrap();

    println!("{}", render(&discounted, product.as_ref()).unwrap());
    // → After 15%  off: 24.65

    // ------------------------------------------------------------------ //
    // 4. String concatenation with +
    // ------------------------------------------------------------------ //
    let user = ctx! {
        "first": "Ada",
        "last":  "Lovelace"
    };

    let full_name = Template::parse("Hello, {{ first + ' ' + last }}!\n").unwrap();
    println!("{}", render(&full_name, user.as_ref()).unwrap());
    // → Hello, Ada Lovelace!

    // ------------------------------------------------------------------ //
    // 5. Parentheses to override precedence
    // ------------------------------------------------------------------ //
    let nums = ctx! { "a": 2, "b": 3, "c": 4 };

    // Without parens: a + b * c = 2 + 12 = 14
    let no_parens = Template::parse("{{ a + b * c }}\n").unwrap();
    // With parens: (a + b) * c = 5 * 4 = 20
    let with_parens = Template::parse("{{ (a + b) * c }}\n").unwrap();

    println!(
        "a + b * c  = {}",
        render(&no_parens, nums.as_ref()).unwrap().trim()
    );
    println!(
        "(a+b) * c  = {}",
        render(&with_parens, nums.as_ref()).unwrap().trim()
    );

    // ------------------------------------------------------------------ //
    // 6. Arithmetic in conditions
    // ------------------------------------------------------------------ //
    let score = ctx! { "score": 73 };

    let grade = Template::parse(concat!(
        "{% if score >= 90 %}A",
        "{% elif score >= 80 %}B",
        "{% elif score >= 70 %}C",
        "{% else %}D{% endif %}\n",
    ))
    .unwrap();

    println!("Grade: {}", render(&grade, score.as_ref()).unwrap().trim());
    // → Grade: C

    // Unary minus
    let delta = ctx! { "value": 5 };
    let neg = Template::parse("{{ -value + 10 }}\n").unwrap();
    println!("-5 + 10 = {}", render(&neg, delta.as_ref()).unwrap().trim());
    // → 5
}
