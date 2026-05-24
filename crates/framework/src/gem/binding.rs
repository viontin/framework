//! GemBinding — standard plug for gems to declare what they wire into the framework.
//!
//! When a gem implements `GemBinding`, the `Boot` builder automatically
//! registers its middlewares, providers, commands, and routes.
//!
//! ```rust
//! impl GemBinding for MyGem {
//!     fn gem_middlewares(&self) -> Vec<Box<dyn Middleware>> {
//!         vec![Box::new(MyAuthMw)]
//!     }
//! }
//! ```

use crate::middleware::Middleware;
use crate::app::ServiceProvider;
use crate::cli::Command;
use crate::server::Router;

use super::GemFacade;

/// Standar colokan antara gem dan framework.
///
/// Gem yang mengimplementasikan trait ini akan otomatis di-wire oleh
/// `Boot::gem()` — middleware, provider, command, dan route akan didaftarkan
/// tanpa perlu panggil `.middleware()` atau `.provider()` secara manual.
pub trait GemBinding: GemFacade {
    /// Middleware yang perlu didaftarkan ke global chain.
    fn gem_middlewares(&self) -> Vec<Box<dyn Middleware + 'static>> { vec![] }

    /// Service provider yang perlu didaftarkan ke container.
    fn gem_providers(&self) -> Vec<Box<dyn ServiceProvider + 'static>> { vec![] }

    /// CLI commands yang perlu didaftarkan ke kernel.
    fn gem_commands(&self) -> Vec<Box<dyn Command + 'static>> { vec![] }

    /// Routes tambahan (static files, SPA fallback, dll).
    fn gem_routes(&self) -> Option<fn(Router) -> Router> { None }
}
