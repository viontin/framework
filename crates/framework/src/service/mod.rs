//! Service layer — business logic with DI and lifecycle hooks.
//!
//! Services orchestrate business logic and coordinate between controllers
//! and repositories. Like `Repository`, they use DI and provide hooks.
//!
//! # Usage
//!
//! ```rust,ignore
//! use viontin_framework::service::{Service, DefaultService};
//! use viontin_orm::{Entity, Repository};
//!
//! // Implement the trait
//! pub struct UserService { pub repo: UserRepo }
//!
//! impl Service<User> for UserService {
//!     fn repo(&self) -> &dyn Repository<User> { &self.repo }
//! }
//!
//! // Or use the default
//! type DefaultUserService = DefaultService<User, UserRepo>;
//! ```

use std::marker::PhantomData;
use crate::entity::Entity;
use crate::repository::Repository;
use crate::db::Value;

/// Service trait — business logic layer with DI and hooks.
///
/// Receives a `Repository` via DI and provides default CRUD operations
/// with `before`/`after` hooks for customization.
///
/// Every method can be overridden. If you don't need a service layer,
/// use `Repository` or `QueryBuilder` directly.
pub trait Service<M: Entity>: std::fmt::Debug + Send + Sync {
    /// The repository this service uses for data access.
    fn repo(&self) -> &dyn Repository<M>;

    // ── Hooks ──

    fn before(&self, _action: &str) -> Result<(), String> { Ok(()) }
    fn after(&self, _action: &str) {}

    // ── Default CRUD ──

    fn all(&self) -> Result<Vec<M>, String> {
        self.before("all")?;
        let r = self.repo().all();
        self.after("all");
        r
    }

    fn find(&self, id: i64) -> Result<Option<M>, String> {
        self.before("find")?;
        let r = self.repo().find(id);
        self.after("find");
        r
    }

    fn create(&self, data: Vec<(&str, Value)>) -> Result<i64, String> {
        self.before("create")?;
        let r = self.repo().create(data);
        self.after("create");
        r
    }

    fn update(&self, id: i64, data: Vec<(&str, Value)>) -> Result<u64, String> {
        self.before("update")?;
        let r = self.repo().update(id, data);
        self.after("update");
        r
    }

    fn delete(&self, id: i64) -> Result<u64, String> {
        self.before("delete")?;
        let r = {
            let entity = self.repo().find(id)?;
            match entity {
                Some(ref e) => self.repo().delete(e),
                None => Ok(0),
            }
        };
        self.after("delete");
        r
    }
}

/// Default service implementation that delegates everything to a repository.
///
/// Useful when you don't need custom business logic yet:
///
/// ```rust,ignore
/// type UserService = DefaultService<User, UserRepo>;
/// ```
#[derive(Debug)]
pub struct DefaultService<M: Entity, R: Repository<M>> {
    pub repo: R,
    _marker: PhantomData<M>,
}

impl<M: Entity, R: Repository<M>> DefaultService<M, R> {
    pub fn new(repo: R) -> Self {
        DefaultService { repo, _marker: PhantomData }
    }
}

impl<M: Entity, R: Repository<M> + 'static> Service<M> for DefaultService<M, R> {
    fn repo(&self) -> &dyn Repository<M> {
        &self.repo
    }
}
