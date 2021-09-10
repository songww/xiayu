//! Error module
use std::{borrow::Cow, fmt, io, num};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseConstraint {
    Fields(Vec<String>),
    Index(String),
    ForeignKey,
    CannotParse,
}

impl DatabaseConstraint {
    pub(crate) fn fields<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        let fields = names.into_iter().map(|s| s.to_string()).collect();

        Self::Fields(fields)
    }
}

impl fmt::Display for DatabaseConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(fields) => write!(f, "({})", fields.join(",")),
            Self::Index(index) => index.fmt(f),
            Self::ForeignKey => "FOREIGN KEY".fmt(f),
            Self::CannotParse => "".fmt(f),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Name {
    Available(String),
    Unavailable,
}

impl Name {
    pub fn available(name: impl ToString) -> Self {
        Self::Available(name.to_string())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available(name) => name.fmt(f),
            Self::Unavailable => write!(f, "(not available)"),
        }
    }
}

impl<T> From<Option<T>> for Name
where
    T: ToString,
{
    fn from(name: Option<T>) -> Self {
        match name {
            Some(name) => Self::available(name),
            None => Self::Unavailable,
        }
    }
}

#[derive(Debug, Error)]
/// The error types for database I/O, connection and query parameter
/// construction.
pub struct Error {
    kind: ErrorKind,
    original_code: Option<String>,
    original_message: Option<String>,
}

pub(crate) struct ErrorBuilder {
    kind: ErrorKind,
    original_code: Option<String>,
    original_message: Option<String>,
}

impl ErrorBuilder {
    pub(crate) fn set_original_code(&mut self, code: impl Into<String>) -> &mut Self {
        self.original_code = Some(code.into());
        self
    }

    pub(crate) fn set_original_message(&mut self, message: impl Into<String>) -> &mut Self {
        self.original_message = Some(message.into());
        self
    }

    pub(crate) fn build(self) -> Error {
        Error {
            kind: self.kind,
            original_code: self.original_code,
            original_message: self.original_message,
        }
    }
}

impl Error {
    pub(crate) fn builder(kind: ErrorKind) -> ErrorBuilder {
        ErrorBuilder {
            kind,
            original_code: None,
            original_message: None,
        }
    }

    /// The error code sent by the database, if available.
    pub fn original_code(&self) -> Option<&str> {
        self.original_code.as_deref()
    }

    /// The original error message sent by the database, if available.
    pub fn original_message(&self) -> Option<&str> {
        self.original_message.as_deref()
    }

    /// A more specific error type for matching.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Determines if the error was associated with closed connection.
    pub fn is_closed(&self) -> bool {
        matches!(self.kind, ErrorKind::ConnectionClosed)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Error occurred while parsing a connection string.
    #[error("{0}")]
    SQLxConfiguration(#[source] sqlx::error::Error),

    /// Error returned from the database.
    #[error("{0}")]
    SQLxDatabase(#[source] sqlx::error::Error),

    /// Error communicating with the database backend./// Error communicating with the database backend.
    #[error("{0}")]
    SQLxIo(#[source] sqlx::error::Error),

    /// Error occurred while attempting to establish a TLS connection.
    #[error("{0}")]
    SQLxTls(#[source] sqlx::error::Error),

    /// Unexpected or invalid data encountered while communicating with the database.
    ///
    /// This should indicate there is a programming error in a SQLx driver or there
    /// is something corrupted with the connection to the database itself.
    #[error("{0}")]
    SQLxProtocol(#[source] sqlx::error::Error),

    /// No rows returned by a query that expected to return at least one row.
    #[error("{0}")]
    NotFound(#[source] sqlx::error::Error),

    /// Type in query doesn't exist. Likely due to typo or missing user type.
    #[error("{0}")]
    SQLxTypeNotFound(#[source] sqlx::error::Error),

    /// Column index was out of bounds.
    #[error("{0}")]
    SQLxColumnIndexOutOfBounds(#[source] sqlx::error::Error),

    /// No column found for the given name.
    #[error("{0}")]
    SQLxColumnNotFound(#[source] sqlx::error::Error),

    /// Error occurred while decoding a value from a specific column.
    #[error("{0}")]
    SQLxColumnDecode(#[source] sqlx::error::Error),

    /// Error occurred while decoding a value.
    #[error("{0}")]
    SQLxDecode(#[source] sqlx::error::Error),

    /// A [`Pool::acquire`] timed out due to connections not becoming available or
    /// because another task encountered too many errors while trying to open a new connection.
    ///
    /// [`Pool::acquire`]: sqlx::pool::Pool::acquire
    #[error("{0}")]
    SQLxPoolTimedOut(#[source] sqlx::error::Error),

    /// [`Pool::close`] was called while we were waiting in [`Pool::acquire`].
    ///
    /// [`Pool::acquire`]: sqlx::pool::Pool::acquire
    /// [`Pool::close`]: sqlx::pool::Pool::close
    #[error("{0}")]
    SQLxPoolClosed(#[source] sqlx::error::Error),

    /// A background worker has crashed.
    #[error("{0}")]
    SQLxWorkerCrashed(#[source] sqlx::error::Error),

    /// Other SQLx error not handled yet.
    #[error("{0}")]
    OtherSQLxError(#[source] sqlx::error::Error),

    #[error("Error querying the database: {}", _0)]
    QueryError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Database does not exist: {}", db_name)]
    DatabaseDoesNotExist { db_name: Name },

    #[error("Access denied to database {}", db_name)]
    DatabaseAccessDenied { db_name: Name },

    #[error("Database already exists {}", db_name)]
    DatabaseAlreadyExists { db_name: Name },

    #[error("Authentication failed for user {}", user)]
    AuthenticationFailed { user: Name },

    #[error("No such table: {}", table)]
    TableDoesNotExist { table: Name },

    #[error("Unique constraint failed: {}", constraint)]
    UniqueConstraintViolation { constraint: DatabaseConstraint },

    #[error("Null constraint failed: {}", constraint)]
    NullConstraintViolation { constraint: DatabaseConstraint },

    #[error("Foreign key constraint failed: {}", constraint)]
    ForeignKeyConstraintViolation { constraint: DatabaseConstraint },

    #[error("Error creating a database connection.")]
    ConnectionError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Error reading the column value: {}", _0)]
    ColumnReadFailure(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("Error accessing result set, index out of bounds: {}", _0)]
    ResultIndexOutOfBounds(usize),

    // #[error("Error accessing result set, column not found: {}", column)]
    // ColumnNotFound { column: Name },
    #[error("Error accessing result set, type mismatch, expected: {}", _0)]
    ResultTypeMismatch(&'static str),

    #[error("Error parsing connection string: {}", _0)]
    DatabaseUrlIsInvalid(String),

    #[error("Conversion failed: {}", _0)]
    ConversionError(Cow<'static, str>),

    #[error("The value provided for column {:?} is too long.", column)]
    LengthMismatch { column: Name },

    #[error("The provided arguments are not supported")]
    InvalidConnectionArguments,

    #[error("Error in an I/O operation: {0}")]
    IoError(io::Error),

    #[error("Timed out when connecting to the database.")]
    ConnectTimeout,

    #[error("The server terminated the connection.")]
    ConnectionClosed,

    #[error(
        "Timed out fetching a connection from the pool (connection limit: {}, in use: {})",
        max_open,
        in_use
    )]
    PoolTimeout { max_open: u64, in_use: u64 },

    #[error("Timed out during query execution.")]
    SocketTimeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },

    #[error("Value out of range error. {}", message)]
    ValueOutOfRange { message: String },

    #[error(
        "Incorrect number of parameters given to a statement. Expected {}: got: {}.",
        expected,
        actual
    )]
    IncorrectNumberOfParameters { expected: usize, actual: usize },
}

impl ErrorKind {
    #[cfg(feature = "mysql")]
    pub(crate) fn value_out_of_range(msg: impl Into<String>) -> Self {
        Self::ValueOutOfRange {
            message: msg.into(),
        }
    }

    pub(crate) fn conversion(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::ConversionError(msg.into())
    }

    #[allow(dead_code)]
    pub(crate) fn database_url_is_invalid(msg: impl Into<String>) -> Self {
        Self::DatabaseUrlIsInvalid(msg.into())
    }

    #[cfg(feature = "pooled")]
    pub(crate) fn pool_timeout(max_open: u64, in_use: u64) -> Self {
        Self::PoolTimeout { max_open, in_use }
    }
}

impl From<Error> for ErrorKind {
    fn from(e: Error) -> Self {
        e.kind
    }
}

/*
#[cfg(feature = "bigdecimal")]
#[cfg_attr(docsrs, doc(cfg(feature = "bigdecimal")))]
impl From<bigdecimal::ParseBigDecimalError> for Error {
    fn from(e: bigdecimal::ParseBigDecimalError) -> Self {
        let kind = ErrorKind::conversion(format!("{}", e));
        Self::builder(kind).build()
    }
}
*/

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::builder(ErrorKind::conversion("Malformed JSON data.")).build()
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        Self::builder(ErrorKind::conversion(
            "Problems writing AST into a query string.",
        ))
        .build()
    }
}

impl From<sqlx::error::Error> for Error {
    fn from(err: sqlx::error::Error) -> Self {
        let kind = match err {
            sqlx::error::Error::Configuration(_) => ErrorKind::SQLxConfiguration(err),
            sqlx::error::Error::Database(_) => ErrorKind::SQLxDatabase(err),
            sqlx::error::Error::Io(_) => ErrorKind::SQLxIo(err),
            sqlx::error::Error::Tls(_) => ErrorKind::SQLxTls(err),
            sqlx::error::Error::Protocol(_) => ErrorKind::SQLxProtocol(err),
            sqlx::error::Error::RowNotFound => ErrorKind::NotFound(err),
            sqlx::error::Error::TypeNotFound { .. } => ErrorKind::SQLxTypeNotFound(err),
            sqlx::error::Error::ColumnIndexOutOfBounds { .. } => {
                ErrorKind::SQLxColumnIndexOutOfBounds(err)
            }
            sqlx::error::Error::ColumnNotFound(_) => ErrorKind::SQLxColumnNotFound(err),
            sqlx::error::Error::ColumnDecode { .. } => ErrorKind::SQLxColumnDecode(err),
            sqlx::error::Error::Decode(_) => ErrorKind::SQLxDecode(err),
            sqlx::error::Error::PoolTimedOut => ErrorKind::SQLxPoolTimedOut(err),
            sqlx::error::Error::PoolClosed => ErrorKind::SQLxPoolClosed(err),
            sqlx::error::Error::WorkerCrashed => ErrorKind::SQLxWorkerCrashed(err),
            _ => ErrorKind::OtherSQLxError(err),
        };
        Self::builder(kind).build()
    }
}

impl From<num::TryFromIntError> for Error {
    fn from(_: num::TryFromIntError) -> Self {
        Self::builder(ErrorKind::conversion(
            "Couldn't convert an integer (possible overflow).",
        ))
        .build()
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::builder(ErrorKind::IoError(e)).build()
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_e: std::num::ParseIntError) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to an integer")).build()
    }
}

impl From<std::str::ParseBoolError> for Error {
    fn from(_e: std::str::ParseBoolError) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to a boolean")).build()
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Error {
        Error::builder(ErrorKind::conversion("Couldn't convert data to UTF-8")).build()
    }
}
