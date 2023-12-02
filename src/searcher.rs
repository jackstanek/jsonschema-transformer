use std::{cmp::Ordering, collections::BTreeMap, ops::*};

use crate::{ir::IR, schema::Schema};

/// Extended natural numbers (naturals plus infinity). Used for edit distances;
/// Inf represents a path that doesn't exist. (i.e. all distances of sound
/// transform paths are of finite length.)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord)]
pub enum ExtNat {
    Nat(u64),
    Inf,
}

impl PartialOrd for ExtNat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use ExtNat::*;
        match (self, other) {
            (Inf, Inf) => None,
            (Nat(_), Inf) => Some(Ordering::Less),
            (Inf, Nat(_)) => Some(Ordering::Greater),
            (Nat(x), Nat(y)) => <u64 as PartialOrd>::partial_cmp(x, y),
        }
    }
}

impl Add for ExtNat {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Inf, _) | (_, Self::Inf) => Self::Inf,
            (Self::Nat(x), Self::Nat(y)) => Self::Nat(x + y),
        }
    }
}

impl AddAssign<u64> for ExtNat {
    fn add_assign(&mut self, rhs: u64) {
        if let Self::Nat(x) = self {
            *x += rhs
        }
    }
}

impl AddAssign for ExtNat {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

pub trait Searcher<T, I, E> {
    fn find_path(&mut self, lhs: &T, rhs: &T) -> Result<Vec<I>, E>;
}

#[derive(Debug)]
pub enum SearchErr {
    NoPath,
}

pub struct SchemaSearcher<'a> {
    schema_rels: BTreeMap<(&'a Schema, &'a Schema), Vec<IR>>,
}

impl<'a> SchemaSearcher<'a> {
    pub fn new() -> Self {
        Self {
            schema_rels: BTreeMap::new(),
        }
    }
}

impl<'a> Searcher<Schema, IR, SearchErr> for SchemaSearcher<'a> {
    fn find_path(&mut self, lhs: &Schema, rhs: &Schema) -> Result<Vec<IR>, SearchErr> {
        use Schema::*;
        use SearchErr::*;
        match self.schema_rels.get(&(lhs, rhs)) {
            Some(p) => Ok(p.clone()),
            None => {
                let path = match (lhs, rhs) {
                    (Ground(g1), Ground(g2)) => {
                        if g1 == g2 {
                            vec![IR::Copy]
                        } else {
                            vec![IR::G2G(*g1, *g2)]
                        }
                    }
                    (Ground(_), Arr(_)) => {
                        return Err(NoPath); // TODO: Implement this?
                    }
                    (Ground(_), Obj(o)) => {
                        if o.keys().len() != 1 {
                            return Err(NoPath);
                        }
                        let (k, v) = o.iter().nth(0).unwrap();

                        let mut path = self.find_path(lhs, v)?;
                        path.push(IR::Abs(k.clone()));
                        path
                    }
                    (Arr(_), Ground(_)) => return Err(NoPath),
                    (Arr(s1), Arr(s2)) => {
                        let mut inner_conv = self.find_path(&s1, &s2)?;
                        let mut path = vec![IR::PushArr];
                        path.append(&mut inner_conv);
                        path.push(IR::PopArr);
                        path
                    }
                    (Arr(_), Obj(_)) => {
                        return Err(NoPath); // TODO: Implement array/object inversion
                    }
                    (Obj(o), Ground(g1)) => {
                        let mut path = Vec::new();
                        for (k, v) in o.iter() {
                            if let Ground(g2) = v.as_ref() {
                                if g1 == g2 {
                                    path.push(IR::Extr(k.clone()));
                                    break;
                                }
                            }
                        }
                        if path.len() > 0 {
                            path
                        } else {
                            return Err(NoPath);
                        }
                    }
                    (Obj(_), Arr(_)) => {
                        return Err(NoPath); // TODO: Implement array/object inversion
                    }
                    (Obj(o1), Obj(o2)) => {
                        let mut path = Vec::new();
                        for k2 in o2.keys() {
                            if !o1.contains_key(k2) {
                                return Err(NoPath);
                            }
                        }

                        path.push(IR::PushObj);
                        for (k1, v1) in o1.iter() {
                            if let Some(v2) = o2.get(k1) {
                                let mut key_conv = self.find_path(v1, v2)?;
                                path.push(IR::PushKey(k1.clone()));
                                path.append(&mut key_conv);
                                path.push(IR::PopKey);
                            }
                        }
                        path.push(IR::PopObj);
                        path
                    }
                    (True, _) | (_, True) => vec![],
                    (False, _) | (_, False) => return Err(NoPath),
                };
                Ok(path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, vec};

    use itertools::iproduct;

    use super::*;
    use crate::{schema, schema::Ground};
    use Ground::*;
    use Schema::*;

    const GROUNDS: [Ground; 4] = [Bool, Num, String, Null];

    macro_rules! assert_path {
        ($from:expr, $to:expr, $expected:expr) => {{
            let mut searcher = SchemaSearcher::new();
            let path = searcher
                .find_path(&$from, &$to)
                .expect("failed to find path");
            assert_eq!(path, $expected)
        }};
    }

    #[test]
    fn test_ground_to_ground() {
        for (from, to) in iproduct!(GROUNDS, GROUNDS) {
            let path = if from != to {
                vec![IR::G2G(from, to)]
            } else {
                vec![IR::Copy]
            };
            assert_path!(Ground(from), Ground(to), path);
        }
    }

    #[test]
    fn test_abstract_into_object() {
        for (from, to) in iproduct!(GROUNDS, GROUNDS) {
            if from == to {
                continue;
            }
            let g2g = IR::G2G(from, to);

            let from = Ground(from);
            let to = Arc::new(Ground(to));

            let key = Arc::new("some_foo_key".to_string());
            let mut map = BTreeMap::new();
            map.insert(key.clone(), to.clone());

            let to = Obj(map);
            let path = vec![g2g, IR::Abs(key)];
            assert_path!(from, to, path);
        }
    }

    #[test]
    fn test_converting_objects() {
        let from = schema!({
            "type": "object",
            "properties": {
                "foo": {
                    "type": "number"
                },
                "bar": {
                    "type": "boolean"
                }
            }
        });
        let to = schema!({
            "type": "object",
            "properties": {
                "foo": {
                    "type": "string"
                },
                "bar": {
                    "type": "boolean"
                }
            }
        });
        let expected = vec![
            IR::PushObj,
            IR::PushKey(Arc::new("bar".to_string())),
            IR::Copy,
            IR::PopKey,
            IR::PushKey(Arc::new("foo".to_string())),
            IR::G2G(Num, String),
            IR::PopKey,
            IR::PopObj,
        ];
        assert_path!(from, to, expected);
    }

    #[test]
    fn test_deleting_key() {
        let from = schema!({
            "type": "object",
            "properties": {
                "foo": {
                    "type": "number"
                },
                "bar": {
                    "type": "boolean"
                }
            }
        });
        let to = schema!({
            "type": "object",
            "properties": {
                "foo": {
                    "type": "string"
                },
            }
        });
        let expected = vec![
            IR::PushObj,
            IR::PushKey(Arc::new("foo".to_string())),
            IR::G2G(Num, String),
            IR::PopKey,
            IR::PopObj,
        ];
        assert_path!(from, to, expected);
    }

    #[test]
    fn test_extracting_key() {
        let from = schema!({
            "type": "object",
            "properties": {
                "foo": {
                    "type": "number"
                }
            }
        });

        let to = schema!({
            "type": "number"
        });

        let expected = vec![IR::Extr(Arc::new("foo".to_string()))];
        assert_path!(from, to, expected);
    }
}
