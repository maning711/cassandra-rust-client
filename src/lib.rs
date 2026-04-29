pub mod client;
pub mod config;
pub mod error;
pub mod repository;
pub mod retry;

pub use client::CassandraClient;
pub use config::{CassandraConfig, ConsistencyLevel, AuthConfig};
pub use error::{CassandraError, Result};
pub use repository::{Repository, QueryBuilder, PoolManager};
pub use retry::{RetryPolicy, CircuitBreaker};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = CassandraConfig::default();
        assert_eq!(config.contact_points.len(), 1);
        assert_eq!(config.connections_per_host, 2);
    }
    
    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .select(&["id", "name", "email"])
            .from("users")
            .where_clause("id = ?")
            .limit(10)
            .build();
        
        assert_eq!(query, "SELECT id, name, email FROM users WHERE id = ? LIMIT 10");
    }
}
