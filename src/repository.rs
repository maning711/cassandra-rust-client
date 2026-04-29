use crate::client::CassandraClient;
use crate::error::Result;
use async_trait::async_trait;

/// Repository trait for data access operations
#[async_trait]
pub trait Repository<T> {
    async fn insert(&self, entity: &T) -> Result<()>;
    async fn find_by_id(&self, id: &str) -> Result<Option<T>>;
    async fn find_all(&self) -> Result<Vec<T>>;
    async fn update(&self, entity: &T) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Query builder for constructing CQL queries
pub struct QueryBuilder {
    query: String,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }
    
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.query = format!("SELECT {}", columns.join(", "));
        self
    }
    
    pub fn from(mut self, table: &str) -> Self {
        self.query = format!("{} FROM {}", self.query, table);
        self
    }
    
    pub fn where_clause(mut self, condition: &str) -> Self {
        self.query = format!("{} WHERE {}", self.query, condition);
        self
    }
    
    pub fn limit(mut self, limit: usize) -> Self {
        self.query = format!("{} LIMIT {}", self.query, limit);
        self
    }
    
    pub fn build(self) -> String {
        self.query
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool manager
pub struct PoolManager {
    client: CassandraClient,
    pool_size: usize,
}

impl PoolManager {
    pub fn new(client: CassandraClient, pool_size: usize) -> Self {
        Self { client, pool_size }
    }
    
    pub fn get_client(&self) -> CassandraClient {
        self.client.clone()
    }
}
