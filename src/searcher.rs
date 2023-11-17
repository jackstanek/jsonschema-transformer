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

struct PathCandidate<T> {
    path: Vec<T>,
    commit_idx: usize,
}

impl<T> PathCandidate<T> {
    fn new() -> Self {
        Self {
            path: Vec::new(),
            commit_idx: 0, // Everything *before* is committed; including and after is provisional
        }
    }

    fn push(&mut self, x: T) {
        self.path.push(x)
    }

    fn rollback_to(&mut self, idx: usize) {
        self.path.drain(idx..);
    }

    fn rollback(&mut self) {
        self.rollback_to(self.commit_idx)
    }

    fn transact<F, E>(&mut self, func: F) -> Result<(), E>
    where
        F: FnOnce() -> Result<(), E>,
    {
        let start = self.path.len();
        func().or_else(|e| {
            self.rollback_to(start);
            Err(e)
        })?;
        self.commit();
        Ok(())
    }

    fn commit(&mut self) {
        self.commit_idx = self.path.len()
    }

    fn finalize(mut self) -> Vec<T> {
        self.rollback();
        self.path
    }
}

trait Searcher<T> {
    fn find_path(self, lhs: &T, rhs: &T) -> Result<Vec<IR>, SearchErr>;
}

pub struct SchemaSearcher {
    path_candidate: PathCandidate<IR>,
    schema_rels: BTreeSet<(Arc<Schema>, Arc<Schema>)>,
}

pub enum SearchErr {
    NoPath,
}

impl Searcher<Schema> for SchemaSearcher {
    fn find_path(mut self, lhs: &Schema, rhs: &Schema) -> Result<Vec<IR>, SearchErr> {
        self.search_paths(lhs, rhs)?;
        Ok(self.path_candidate.finalize())
    }
}

impl SchemaSearcher {
    pub fn new() -> Self {
        Self {
            path_candidate: PathCandidate::new(),
            schema_rels: BTreeSet::new(),
        }
    }

    pub fn search_paths(&mut self, lhs: &Schema, rhs: &Schema) -> Result<(), SearchErr> {
        use Schema::*;
        use SearchErr::NoPath;

        if lhs == rhs {
            return Ok(());
        }

        match (lhs, rhs) {
            // convert an array
            (Arr(s1), Arr(s2)) => {
                self.path_candidate.push(IR::PushArr);
                if let Err(e) = self.search_paths(s1, s2) {
                    self.path_candidate.rollback();
                    return Err(e);
                }
                self.path_candidate.push(IR::Pop);
                self.path_candidate.commit();
                Ok(())
            }
            // convert an object property-wise
            (Obj(o1), Obj(o2)) => {
                for k in o2.keys() {
                    o1.get(k).ok_or(NoPath)?;
                }

                for (k, v1) in o1.iter() {
                    match o2.get(k) {
                        None => self.path_candidate.push(IR::Del(k.clone())),
                        Some(v2) => {
                            self.path_candidate.push(IR::PushObj(k.clone()));
                            self.search_paths(&v1, &v2)?;
                            self.path_candidate.push(IR::Pop);
                        }
                    }
                }
                Ok(())
            }
            // extract single property from object
            (Obj(o1), v2) => {
                return o1
                    .iter()
                    .find(|(k, v)| v.as_ref() == v2)
                    .ok_or(NoPath)
                    .and_then(|(k, _)| Ok(self.path_candidate.push(IR::Extr(k.clone()))));
            }
            (Ground(g1), Ground(g2)) => {
                self.path_candidate.push(IR::G2G(*g1, *g2));
                Ok(())
            }
            (True, _) | (_, True) => Ok(()),
            (_, _) => Err(NoPath),
        }
    }
}

pub struct SchemaSearcher2<'a> {
    schema_rels: BTreeMap<(&'a Schema, &'a Schema), Vec<IR>>,
}

impl<'a> SchemaSearcher2<'a> {
    fn find_path(&mut self, lhs: &Schema, rhs: &Schema) -> Result<&'a Vec<IR>, SearchErr> {
        use Schema::*;
        use SearchErr::*;

        if let Some(p) = self.schema_rels.get(&(lhs, rhs)) {
            return Ok(p)
        }

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
                path.clone_from(self.find_path(v.as_ref(), rhs)?);
                path
            }
            (Arr(_), Ground(_)) => return Err(NoPath),
            (Arr(s1), Arr(s2)) => {
                let inner_conv = self.find_path(&s1, &s2)?;
                let mut path = vec![IR::PushArr];
                path.clone_from(inner_conv);
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
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod path_candidate {
        use super::*;
        #[test]
        fn test_push_to_path_candidate() {}
    }
}
