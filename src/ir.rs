use std::sync::Arc;

use crate::schema::Ground;

/// IR for schema transformers
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IR {
    G2G(Ground, Ground),
    Del(Arc<String>),
    PushArr,
    PopArr,
    PushObj,
    PopObj,
    PushKey(Arc<String>),
    PopKey,
    Abs(Arc<String>),
    Extr(Arc<String>),
    Inv,
}

