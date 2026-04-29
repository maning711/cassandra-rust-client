use serde::{Deserialize, Serialize};

/// Cassandra client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CassandraConfig {
    /// Contact points (nodes) to connect to
    pub contact_points: Vec<String>,
    
    /// Keyspace to use
    pub keyspace: Option<String>,
    
    /// Default consistency level
    pub consistency: ConsistencyLevel,
    
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    
    /// Number of connections per host
    pub connections_per_host: usize,
    
    /// Enable compression
    pub compression: bool,
    
    /// Authentication credentials
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Any,
    One,
    Two,
    Three,
    Quorum,
    All,
    LocalQuorum,
    EachQuorum,
}

impl Default for CassandraConfig {
    fn default() -> Self {
        Self {
            contact_points: vec!["127.0.0.1:9042".to_string()],
            keyspace: None,
            consistency: ConsistencyLevel::Quorum,
            connection_timeout_ms: 5000,
            request_timeout_ms: 10000,
            connections_per_host: 2,
            compression: true,
            auth: None,
        }
    }
}

impl From<ConsistencyLevel> for scylla::statement::Consistency {
    fn from(level: ConsistencyLevel) -> Self {
        match level {
            ConsistencyLevel::Any => scylla::statement::Consistency::Any,
            ConsistencyLevel::One => scylla::statement::Consistency::One,
            ConsistencyLevel::Two => scylla::statement::Consistency::Two,
            ConsistencyLevel::Three => scylla::statement::Consistency::Three,
            ConsistencyLevel::Quorum => scylla::statement::Consistency::Quorum,
            ConsistencyLevel::All => scylla::statement::Consistency::All,
            ConsistencyLevel::LocalQuorum => scylla::statement::Consistency::LocalQuorum,
            ConsistencyLevel::EachQuorum => scylla::statement::Consistency::EachQuorum,
        }
    }
}
