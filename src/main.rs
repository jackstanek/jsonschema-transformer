use std::fmt::Display;

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

#[derive(Debug)]
enum Transformer {
    Num2Bool,
    RemoveKey,
    InvertList,
}

impl Display for Transformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

fn main() {
    use Transformer::*;
    let rules: Vec<Rewrite<Schema, ()>> = vec![
        rw!(Num2Bool; "num" => "bool"),
        rw!(RemoveKey; "(obj ?x ?y)" => "?y"),
        rw!(InvertList; "(arr (obj (pair ?x ?y) ?z))" => "(obj (pair ?x (arr ?y)) ?z)")
    ];
    let lexpr: RecExpr<Schema> = "(arr (obj (pair foo num) (obj (pair bar bool) empty)))".parse().unwrap();
    let rexpr: RecExpr<Schema> = "(obj (pair foo (arr num)) (obj (pair bar (arr bool)) empty))".parse().unwrap();
    let mut runner = Runner::<Schema, ()>::default()
        .with_explanations_enabled()
        .with_expr(&lexpr)
        .run(&rules);

    if !runner.egraph.equivs(&lexpr, &rexpr).is_empty() {
        let mut expl = runner
            .explain_equivalence(&lexpr, &rexpr);
        println!("{}", expl.get_flat_string())
    } else {
        println!("Cannot synthesize transformer between schemas");
    }
}
