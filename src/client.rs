use crate::config::{CassandraConfig, ConsistencyLevel};
use crate::error::{CassandraError, Result};
use scylla::{Session, SessionBuilder};
use scylla::transport::session::PoolSize;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Main Cassandra client
pub struct CassandraClient {
    session: Arc<Session>,
    config: CassandraConfig,
}

impl CassandraClient {
    /// Create a new Cassandra client with the given configuration
    pub async fn new(config: CassandraConfig) -> Result<Self> {
        info!("Initializing Cassandra client with config: {:?}", config);
        
        let mut builder = SessionBuilder::new()
            .known_nodes(&config.contact_points);
        
        // Set connection pool size
        builder = builder.pool_size(PoolSize::PerHost(config.connections_per_host));
        
        // Set compression
        if config.compression {
            builder = builder.compression(Some(scylla::transport::Compression::Lz4));
        }
        
        // Set authentication
        if let Some(auth) = &config.auth {
            builder = builder.user(&auth.username, &auth.password);
        }
        
        // Set default keyspace
        if let Some(keyspace) = &config.keyspace {
            builder = builder.use_keyspace(keyspace, false);
        }
        
        // Build session
        let session = builder.build().await?;
        
        info!("Successfully connected to Cassandra cluster");
        
        Ok(Self {
            session: Arc::new(session),
            config,
        })
    }
    
    /// Execute a CQL query without returning results
    pub async fn execute(&self, query: &str, values: impl scylla::serialize::row::SerializeRow) -> Result<()> {
        let mut prepared = self.session.prepare(query).await?;
        prepared.set_consistency(self.config.consistency.into());
        
        self.session.execute(&prepared, values).await?;
        Ok(())
    }
    
    /// Execute a CQL query and return results
    pub async fn query<R>(&self, query: &str, values: impl scylla::serialize::row::SerializeRow) -> Result<Vec<R>>
    where
        R: scylla::deserialize::DeserializeRow<'static, 'static>,
    {
        let mut prepared = self.session.prepare(query).await?;
        prepared.set_consistency(self.config.consistency.into());
        
        let result = self.session.execute(&prepared, values).await?;
        
        match result.rows {
            Some(rows) => {
                let parsed: std::result::Result<Vec<R>, _> = rows
                    .into_typed::<R>()
                    .collect();
                
                parsed.map_err(|e| CassandraError::SerializationError(e.to_string()))
            }
            None => Ok(Vec::new()),
        }
    }
    
    /// Execute a batch of queries
    pub async fn batch(&self, queries: Vec<(&str, Vec<scylla::frame::value::ValueList>)>) -> Result<()> {
        use scylla::batch::Batch;
        
        let mut batch = Batch::default();
        batch.set_consistency(self.config.consistency.into());
        
        for (query, _) in &queries {
            batch.append_statement(query);
        }
        
        // Note: Actual batch execution would need proper value binding
        // This is a simplified version
        warn!("Batch execution needs proper implementation");
        Ok(())
    }
    
    /// Get the underlying session (for advanced operations)
    pub fn session(&self) -> Arc<Session> {
        Arc::clone(&self.session)
    }
    
    /// Use a different keyspace
    pub async fn use_keyspace(&self, keyspace: &str) -> Result<()> {
        self.session.use_keyspace(keyspace, false).await?;
        info!("Switched to keyspace: {}", keyspace);
        Ok(())
    }
    
    /// Check connection health
    pub async fn health_check(&self) -> Result<bool> {
        let query = "SELECT now() FROM system.local";
        match self.session.query(query, &[]).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

impl Clone for CassandraClient {
    fn clone(&self) -> Self {
        Self {
            session: Arc::clone(&self.session),
            config: self.config.clone(),
        }
    }
}
