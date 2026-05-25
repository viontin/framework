//! Service layer — business logic with DI and lifecycle hooks.
//!
//! Services orchestrate business logic and coordinate between controllers
//! and repositories. Like `Repository`, they use DI and provide hooks.
//!
//! ## Sub-modules
//!
//! - `contracts` — ServiceContract, ServiceRegistry, RemoteServiceAdapter

pub mod contracts;

#[cfg(feature = "orm")]
use crate::{Entity, InternalError, InternalResult, Value};
#[cfg(feature = "orm")]
use crate::repositories::Repository;

/// Business logic layer between controllers and repositories.
///
/// A `Service` delegates persistence to a `Repository` and adds
/// business logic, validation, and cross-cutting concerns.
#[cfg(feature = "orm")]
pub trait Service<M: Entity>: std::fmt::Debug + Send + Sync {
    fn repo(&self) -> &dyn Repository<M>;

    /// Hook called before every action. Return Err to abort.
    fn before(&self, _action: &str) -> InternalResult<()> { Ok(()) }

    /// Hook called after every successful action.
    fn after(&self, _action: &str) -> InternalResult<()> { Ok(()) }

    fn all(&self) -> InternalResult<Vec<M>> {
        self.before("all")?;
        let result = self.repo().all().map_err(|e| InternalError::internal(e))?;
        self.after("all")?;
        Ok(result)
    }

    fn find(&self, id: i64) -> InternalResult<Option<M>> {
        self.before("find")?;
        let result = self.repo().find(id).map_err(|e| InternalError::internal(e))?;
        self.after("find")?;
        Ok(result)
    }

    fn create(&self, data: Vec<(&str, Value)>) -> InternalResult<i64> {
        self.before("create")?;
        let id = self.repo().create(data).map_err(|e| InternalError::internal(e))?;
        self.after("create")?;
        Ok(id)
    }

    fn update(&self, id: i64, data: Vec<(&str, Value)>) -> InternalResult<u64> {
        self.before("update")?;
        let affected = self.repo().update(id, data).map_err(|e| InternalError::internal(e))?;
        self.after("update")?;
        Ok(affected)
    }

    fn delete(&self, id: i64) -> InternalResult<u64> {
        self.before("delete")?;
        let entity = self.repo().find(id).map_err(|e| InternalError::internal(e))?
            .ok_or_else(|| InternalError::not_found(format!("entity {}", id)))?;
        let affected = self.repo().delete(&entity).map_err(|e| InternalError::internal(e))?;
        self.after("delete")?;
        Ok(affected)
    }
}

/// Default implementation with full delegation.
#[cfg(feature = "orm")]
#[derive(Debug)]
pub struct DefaultService<M: Entity, R: Repository<M>> {
    repo: R,
    _marker: std::marker::PhantomData<M>,
}

#[cfg(feature = "orm")]
impl<M: Entity, R: Repository<M>> DefaultService<M, R> {
    pub fn new(repo: R) -> Self {
        DefaultService { repo, _marker: std::marker::PhantomData }
    }
}

#[cfg(feature = "orm")]
impl<M: Entity, R: Repository<M>> Service<M> for DefaultService<M, R> {
    fn repo(&self) -> &dyn Repository<M> { &self.repo }
}
