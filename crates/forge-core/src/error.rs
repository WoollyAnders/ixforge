//! The single error type shared across IX Forge crates.

use thiserror::Error;

/// Errors produced anywhere in the device pipeline.
///
/// I/O backends stringify their native errors into the variants below so that
/// `forge-core` stays free of backend-specific dependencies.
#[derive(Debug, Error)]
pub enum ForgeError {
    /// The device or driver does not implement the requested capability.
    #[error("operation not supported by this device")]
    NotSupported,

    /// No attached device matched the given identifier.
    #[error("device not found: {0}")]
    DeviceNotFound(String),

    /// The transport/backend failed (USB error, permission denied, disconnect).
    #[error("transport error: {0}")]
    Transport(String),

    /// The device replied unexpectedly or a packet could not be (de)coded.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// A device profile failed to parse or validate.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),

    /// A caller supplied an out-of-range or malformed argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Filesystem or serialization failure.
    #[error("i/o error: {0}")]
    Io(String),
}

/// Convenience alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, ForgeError>;
