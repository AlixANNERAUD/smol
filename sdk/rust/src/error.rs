//! Errors surfaced by the SDK.
//!
//! The engine's error enum is large and carries internal variants that mean
//! nothing to an embedder. This module collapses it into a small [`ErrorKind`]
//! that callers can actually match on, keeping the engine's own message as the
//! payload. The kinds line up one-for-one with the string codes the Node and
//! Python SDKs raise, so the three SDKs agree on what a failure is called.

use std::fmt;

/// What went wrong, in terms an embedder can branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A machine, image, rootfs, disk or mount source does not exist.
    NotFound,
    /// The machine is not in a state that permits the operation.
    InvalidState,
    /// No hypervisor is available on this host.
    HypervisorUnavailable,
    /// KVM is missing, or the process may not open it.
    KvmUnavailable,
    /// The name is taken, or the guest rejected the request as conflicting.
    Conflict,
    /// A disk, overlay or image store operation failed.
    Storage,
    /// A mount could not be set up, or its path is not usable.
    Mount,
    /// The machine configuration is not valid.
    Config,
    /// A command ran but failed.
    CommandFailed,
    /// The credential was missing, rejected or expired.
    Unauthorized,
    /// The request could not be delivered.
    Connection,
    /// The request outlived its deadline.
    Timeout,
    /// The operation exists, but not on this target.
    NotSupported,
    /// The machine stopped or became unreachable while a command ran in it.
    /// Retrying can succeed, but the command may have partly run.
    MachineInterrupted,
    /// Anything the SDK does not classify further.
    Other,
}

impl ErrorKind {
    /// The stable string code, matching the Node and Python SDKs.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::InvalidState => "INVALID_STATE",
            Self::HypervisorUnavailable => "HYPERVISOR_UNAVAILABLE",
            Self::KvmUnavailable => "KVM_UNAVAILABLE",
            Self::Conflict => "CONFLICT",
            Self::Storage => "STORAGE_ERROR",
            Self::Mount => "MOUNT_ERROR",
            Self::Config => "CONFIG_ERROR",
            Self::CommandFailed => "COMMAND_FAILED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Connection => "CONNECTION",
            Self::Timeout => "TIMEOUT",
            Self::NotSupported => "NOT_SUPPORTED",
            Self::MachineInterrupted => "MACHINE_INTERRUPTED",
            Self::Other => "SMOLVM_ERROR",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// An SDK failure: a [`ErrorKind`] plus the engine's own description.
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    retryable: bool,
    retry_after: Option<std::time::Duration>,
}

impl Error {
    /// Build an error directly. Mostly useful in tests.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable: false,
            retry_after: None,
        }
    }

    /// Whether sending the same request again can succeed, as the cloud API
    /// reported it. A command whose machine went away is retryable, but it
    /// may have partly run, so the SDK leaves that retry to the caller.
    pub fn retryable(&self) -> bool {
        self.retryable
    }

    /// How long the cloud API asked callers to wait before retrying.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        self.retry_after
    }

    /// What class of failure this is.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The stable string code for this failure.
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// The engine's description, without the code prefix.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind.code(), self.message)
    }
}

impl std::error::Error for Error {}

impl From<smol_cloud::Error> for Error {
    fn from(err: smol_cloud::Error) -> Self {
        let kind = match err.kind() {
            smol_cloud::ErrorKind::NotFound => ErrorKind::NotFound,
            smol_cloud::ErrorKind::Unauthorized => ErrorKind::Unauthorized,
            smol_cloud::ErrorKind::Conflict => ErrorKind::Conflict,
            smol_cloud::ErrorKind::Timeout => ErrorKind::Timeout,
            smol_cloud::ErrorKind::Connection => ErrorKind::Connection,
            smol_cloud::ErrorKind::MachineInterrupted => ErrorKind::MachineInterrupted,
            _ => ErrorKind::Other,
        };
        // Keep the correlation id in the message: a caller sees the message,
        // never the response headers, and support needs that id to find the
        // call.
        let message = match err.request_id() {
            Some(request_id) => format!("{} [request id: {request_id}]", err.message()),
            None => err.message().to_string(),
        };
        Self {
            kind,
            message,
            retryable: err.retryable(),
            retry_after: err.retry_after(),
        }
    }
}

/// The SDK's result alias.
pub type Result<T> = std::result::Result<T, Error>;
