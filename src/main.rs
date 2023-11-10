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

fn main() -> Result<(), std::io::Error> {
    let s1_path = std::env::args().nth(1).expect("need first argument");
    let s2_path = std::env::args().nth(2).expect("need second argument");

    let s1_json: serde_json::Value =
        serde_json::from_str(std::fs::read_to_string(s1_path)?.as_str())
            .expect("first schema has valid JSON");
    let s2_json: serde_json::Value =
        serde_json::from_str(std::fs::read_to_string(s2_path)?.as_str())
            .expect("second schema has valid JSON");

    let s1 = schema::Schema::try_from(&s1_json).expect("first schema valid");
    let s2 = schema::Schema::try_from(&s2_json).expect("first schema valid");

    println!("edit distance between schemas: {:?}", s1.edit_distance(&s2));
    Ok(())
}
