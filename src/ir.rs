use std::sync::Arc;

use crate::schema::Ground;

/// IR for schema transformers
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IR {
    G2G(Ground, Ground),
    Del(Arc<String>),
    PushArr,
    PushObj(Arc<String>),
    Abs(Arc<String>),
    Extr(Arc<String>),
    Inv,
    Pop,
}

