use std::fmt::Display;

use egg::{rewrite as rw, *};

mod schema;

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
    // Num2Bool,
    // RemoveKey,
    // InvertList,
    ReorderKeys,
}

impl Display for Transformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

fn main() {
    use Transformer::*;
    let rules: Vec<Rewrite<Schema, ()>> = vec![
        //rw!(InvertList; "(obj (pair ?x (arr ?y)) ?z)" => "(obj"),
        //rw!(Num2Bool; "num" => "bool"),
        rw!(ReorderKeys; "(obj (pair ?x ?y) (obj (pair ?z ?w) ?a))" => "(obj (pair ?z ?w) (obj (pair ?x ?y) ?a))"),
    ];
    let lexpr: RecExpr<Schema> = "(obj (pair foo num) (obj (pair bar bool) (obj (pair baz str) empty)))".parse().unwrap();
    let rexpr: RecExpr<Schema> = "(obj (pair baz str) (obj (pair foo num) (obj (pair bar bool) empty)))".parse().unwrap();
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
