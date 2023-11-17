use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    ops::*,
    path::Path,
    sync::Arc,
};

use crate::{
    ir::IR,
    schema::{self, Ground, Schema, SchemaErr},
};

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

trait Searcher<T, I, E> {
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
                            vec![]
                        } else {
                            vec![IR::G2G(*g1, *g2)]
                        }
                    }
                    (Ground(_), Arr(_)) => {
                        return Err(NoPath); // TODO: Implement this?
                    }
                    (Ground(g), Obj(o)) => {
                        if o.keys().len() != 1 {
                            return Err(NoPath);
                        }
                        let (k, v) = o.iter().nth(0).unwrap();
                        let mut path = vec![IR::Abs(k.clone())];
                        path.append(&mut self.find_path(v.as_ref(), rhs)?);
                        path
                    }
                    (Arr(_), Ground(_)) => return Err(NoPath),
                    (Arr(s1), Arr(s2)) => {
                        let mut inner_conv = self.find_path(&s1, &s2)?;
                        let mut path = vec![IR::PushArr];
                        path.append(&mut inner_conv);
                        path.push(IR::Pop);
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
                    (Obj(_), Arr(_)) => todo!(),
                    (Obj(_), Obj(_)) => todo!(),
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
    use itertools::iproduct;

    use super::*;
    use crate::schema::Ground::*;
    use Schema::*;

    #[test]
    fn test_ground_to_ground() {
        for (from, to) in iproduct!([Bool, Num, String, Null], [Bool, Num, String, Null]) {
            let mut searcher = SchemaSearcher::new();
            assert_eq!(
                if from != to {
                    vec![IR::G2G(from, to)]
                } else {
                    Vec::new()
                },
                searcher
                    .find_path(&Schema::Ground(from), &Schema::Ground(to))
                    .expect("found path")
            );
        }
    }
}
