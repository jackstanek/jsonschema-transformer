use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Schema {
    Bool,
    Arr(Arc<Schema>),
    Obj(BTreeMap<Arc<str>, Arc<Schema>>),
}

impl Schema {
    fn edit_distance(&self, other: &Self) -> u64 {
        use Schema::*;

        if self == other {
            return 0;
        }

        match (self, other) {
            (Arr(s1), Arr(s2)) => s1.edit_distance(s2),
            (_, _) => 1,
        }
    }
}
