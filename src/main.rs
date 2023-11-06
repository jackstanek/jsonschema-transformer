use egg::{rewrite as rw, *};

define_language! {
    enum Schema {
        "bool"  = Bool,
        "num"   = Num,
        "null"  = Null,
        "str"   = Str,
        "arr"   = Arr(Id),
        "obj"   = Obj([Id; 2]),
        "pair"  = Pair([Id; 2]),
        "empty" = Empty,
        Key(Symbol),
    }
}

fn main() {
    let mut rules: Vec<Rewrite<Schema, ()>> = vec![
        rw!("num->bool"; "num" => "bool"),
        rw!("remove-key"; "(obj ?x ?y)" => "?y"),
    ];
    let lexpr: RecExpr<Schema> = "(obj (pair foo num) empty)".parse().unwrap();
    let rexpr: RecExpr<Schema> = "(obj (pair foo bool) empty)".parse().unwrap();
    let mut runner = Runner::<Schema, ()>::default()
        .with_explanations_enabled()
        .with_expr(&lexpr)
        .run(&rules);

    if !runner.egraph.equivs(&lexpr, &rexpr).is_empty() {
        println!("{:?}", runner.explain_equivalence(&lexpr, &rexpr).make_flat_explanation());
    } else {
        println!("Not equivalent")
    }
}
