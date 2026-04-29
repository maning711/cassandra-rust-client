use thiserror::Error;

#[derive(Error, Debug)]
pub enum CassandraError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Query execution error: {0}")]
    QueryError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<scylla::transport::errors::NewSessionError> for CassandraError {
    fn from(err: scylla::transport::errors::NewSessionError) -> Self {
        CassandraError::ConnectionError(err.to_string())
    }
}

impl From<scylla::transport::errors::QueryError> for CassandraError {
    fn from(err: scylla::transport::errors::QueryError) -> Self {
        CassandraError::QueryError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CassandraError>;
