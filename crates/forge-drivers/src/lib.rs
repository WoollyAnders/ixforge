//! Per-controller-family protocol drivers.
//!
//! Each family lives in its own module behind a feature flag, so a slim build can
//! ship only the drivers it has tested. Adding a *model* in a known family is a
//! profile file; adding a *family* is a new module here.

use forge_core::Driver;

#[cfg(any(feature = "sinowealth", feature = "sonix"))]
mod framing;

#[cfg(feature = "sinowealth")]
pub mod sinowealth;

#[cfg(feature = "sonix")]
pub mod sonix;

/// Every driver compiled into this build.
#[allow(unused_mut, clippy::vec_init_then_push)] // entries are cfg-gated by feature
pub fn all_drivers() -> Vec<Box<dyn Driver>> {
    let mut drivers: Vec<Box<dyn Driver>> = Vec::new();
    #[cfg(feature = "sinowealth")]
    drivers.push(Box::new(sinowealth::SinoWealthDriver));
    #[cfg(feature = "sonix")]
    drivers.push(Box::new(sonix::SonixDriver));
    drivers
}

/// Find a compiled-in driver by its family key.
pub fn driver_for_family(family: &str) -> Option<Box<dyn Driver>> {
    all_drivers().into_iter().find(|d| d.family() == family)
}
