use std::fmt;

/// The common error type shared across every crate that depends on `ccore`.
///
/// Individual crates are free to define their own richer, domain-specific
/// error types and convert into `CoreError` at their boundary (or vice
/// versa via `From`), but `CoreError` is what crates reach for when they
/// don't need a variant of their own.
#[derive(Debug)]
pub enum CoreError {
    /// Io error
    Io(String),
    /// Invalid Argument error
    InvalidArgument(String),
    /// Invalid State error
    InvalidState(String),
    /// Unsupported error
    Unsupported(String),
    /// Already exists error
    AlreadyExists(String),
    /// Not found error
    NotFound(String),
    /// Time out error
    Timeout(String),
    /// Other errors
    Other(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io(msg) => write!(f, "io error: {msg}"),
            CoreError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            CoreError::InvalidState(msg) => write!(f, "invalid state: {msg}"),
            CoreError::Unsupported(msg) => write!(f, "unsupported: {msg}"),
            CoreError::AlreadyExists(msg) => write!(f, "already exists: {msg}"),
            CoreError::NotFound(msg) => write!(f, "not found: {msg}"),
            CoreError::Timeout(msg) => write!(f, "timeout: {msg}"),
            CoreError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Io(err.to_string())
    }
}

/// Shorthand for `Result<T, CoreError>`.
pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_readable_message() {
        let err = CoreError::NotFound("asset 'grass.png'".into());
        assert_eq!(err.to_string(), "not found: asset 'grass.png'");
    }

    #[test]
    fn converts_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let err: CoreError = io_err.into();
        matches!(err, CoreError::Io(_));
    }
}
