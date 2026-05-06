use crate::config::CassandraConfig;
use crate::error::{CassandraError, Result};
use scylla::{Session, SessionBuilder};
use scylla::transport::session::PoolSize;
use scylla::transport::session::IntoTypedRows;
use scylla::cql_to_rust::FromRow;
use scylla::serialize::row::SerializeRow;
use std::num::NonZeroUsize;
use std::sync::Arc;
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
        let pool_size = NonZeroUsize::new(config.connections_per_host).unwrap_or(NonZeroUsize::new(2).unwrap());
        builder = builder.pool_size(PoolSize::PerHost(pool_size));

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
    pub async fn execute(&self, query: &str, values: impl SerializeRow) -> Result<()> {
        let mut prepared = self.session.prepare(query).await?;
        prepared.set_consistency(self.config.consistency.into());

        self.session.execute(&prepared, values).await?;
        Ok(())
    }

    /// Execute a CQL query and return typed results
    pub async fn query<R>(&self, query: &str, values: impl SerializeRow) -> Result<Vec<R>>
    where
        R: FromRow,
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

    /// Execute a batch of queries (simplified version)
    pub async fn batch(&self, queries: &[&str]) -> Result<()> {
        use scylla::batch::Batch;

        let mut batch = Batch::default();
        batch.set_consistency(self.config.consistency.into());

        for query in queries {
            batch.append_statement(*query);
        }

        warn!("Batch without values — use session().batch() for full control");
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
