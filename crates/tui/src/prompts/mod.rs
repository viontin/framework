//! Interactive terminal prompts — inspired by Laravel Prompts.
//!
//! These prompts require the `prompts` feature (enabled by default) which
//! pulls in `crossterm` for raw terminal input.

mod text;
mod select;
mod confirm;
mod password;

pub use text::text;
pub use select::select;
pub use confirm::confirm;
pub use password::password;

/// Re-export for custom prompt building.
pub use crossterm;
