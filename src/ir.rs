use std::sync::Arc;

use crate::schema::Ground;

/// IR for schema transformers
pub enum IR {
    G2G(Ground, Ground),
    PushArr,
    PushObj(Arc<String>),
    Abs,
    Del,
    Inv,
    Pop,
}

trait Codegen {
    type Output: Into<String>;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output;
}
