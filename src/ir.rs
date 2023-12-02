use std::sync::Arc;

use crate::schema::Ground;

/// IR for schema transformers
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IR {
    G2G(Ground, Ground),
    PushArr,
    PopArr,
    PushObj,
    PopObj,
    PushKey(Arc<String>),
    PopKey,
    Copy,
    Abs(Arc<String>),
    Extr(Arc<String>),
    Inv,
}

