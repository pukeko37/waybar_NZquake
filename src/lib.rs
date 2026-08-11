//! Library surface for `waybar_nzquake`: the domain/app/infra ring
//! layering, exposed so `tests/` can exercise integration tests against
//! the real public API surface — the `waybar_nzquake` binary (`main.rs`)
//! is a thin composition-root wrapper over this.

pub mod app;
pub mod domain;
pub mod infra;
