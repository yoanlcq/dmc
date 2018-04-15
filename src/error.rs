//! `Error` and `Result` types for this crate.
use std::fmt::{self, Display, Formatter};

pub(crate) type CowStr = ::std::borrow::Cow<'static, str>;

/// Different kinds of errors reported by most faillible operations.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ErrorKind {
    /// Operation not supported for target platform / current build settings.
    ///
    /// For instance, trying to open more than one window on targets that don't support it.
    Unsupported,
    /// Some arguments were invalid; You could retry with different ones.
    InvalidArgument,
    /// Operation is supported and arguments were valid, but the operation failed for other
    /// reasons (e.g user-specific environment).
    ///
    /// For instance, on X11-based targets, the user's X11 server may lack some required extensions.
    Failed,
}

/// An `ErrorKind` packed with an optional `reason` string.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Error {
    /// The error kind.
    pub kind: ErrorKind,
    /// A hopefully useful reason string, or `None` if unknown or not meaningful.
    pub reason: Option<CowStr>,
}

/// Alias to `Result<T, Error>`.
pub type Result<T> = ::std::result::Result<T, Error>;

impl ErrorKind {
    pub(crate) fn describe_quick(&self) -> &'static str {
        match *self {
            ErrorKind::InvalidArgument => "Invalid argument(s)",
            ErrorKind::Unsupported => "Unsupported operation for target platform",
            ErrorKind::Failed => "Operation has failed",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.describe_quick())
    }
}

impl ::std::error::Error for ErrorKind {
    fn description(&self) -> &str {
        self.describe_quick()
    }
}


impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.kind.describe_quick())?;
        match self.reason {
            None => write!(f, " (no reason given)"),
            Some(ref s) => write!(f, ": {}", s),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        self.kind.describe_quick()
    }
}

#[allow(unused_imports)]
pub(crate) use self::utils::*;

mod utils {
    #![allow(dead_code)]
    use super::*;

    impl Error {
        pub(crate) fn unsupported<S: Into<CowStr>>(s: S) -> Self {
            Self { kind: ErrorKind::Unsupported, reason: Some(s.into()), }
        }
        pub(crate) fn invalid_arg<S: Into<CowStr>>(s: S) -> Self {
            Self { kind: ErrorKind::InvalidArgument, reason: Some(s.into()), }
        }
        pub(crate) fn failed<S: Into<CowStr>>(s: S) -> Self {
            Self { kind: ErrorKind::Failed, reason: Some(s.into()), }
        }
        pub(crate) fn unsupported_unexplained() -> Self {
            Self { kind: ErrorKind::Unsupported, reason: None, }
        }
        pub(crate) fn invalid_arg_unexplained() -> Self {
            Self { kind: ErrorKind::InvalidArgument, reason: None, }
        }
        pub(crate) fn failed_unexplained() -> Self {
            Self { kind: ErrorKind::Failed, reason: None, }
        }
    }

    pub(crate) fn unsupported<T, S: Into<CowStr>>(s: S) -> self::Result<T> {
        Err(Error::unsupported(s))
    }
    pub(crate) fn invalid_arg<T, S: Into<CowStr>>(s: S) -> self::Result<T> {
        Err(Error::invalid_arg(s))
    }
    pub(crate) fn failed<T, S: Into<CowStr>>(s: S) -> self::Result<T> {
        Err(Error::failed(s))
    }
    pub(crate) fn unsupported_unexplained<T>() -> self::Result<T> {
        Err(Error::unsupported_unexplained())
    }
    pub(crate) fn invalid_arg_unexplained<T>() -> self::Result<T> {
        Err(Error::invalid_arg_unexplained())
    }
    pub(crate) fn failed_unexplained<T>() -> self::Result<T> {
        Err(Error::failed_unexplained())
    }
}
