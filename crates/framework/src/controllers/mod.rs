//! Controller — HTTP request handler.
//!
//! A `Controller` maps an HTTP request to a response.
//! For ORM-backed CRUD defaults, enable the `orm` feature.

use crate::http::{Request, Response};

pub trait Controller: std::fmt::Debug + Send + Sync + 'static {
    fn handle(&self, req: &Request) -> Response;
}

#[cfg(feature = "orm")]
pub mod defaults;
#[cfg(feature = "orm")]
pub use defaults::{HandlesCrud, DefaultController};
