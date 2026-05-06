use cassandra_rust_client::{
    CassandraClient, CassandraConfig, ConsistencyLevel, Repository, QueryBuilder, RetryPolicy
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: Uuid,
    name: String,
    email: String,
    age: i32,
}

impl scylla::serialize::row::SerializeRow for User {
    fn serialize(&self, _ctx: &scylla::serialize::row::RowSerializationContext<'_>, _writer: &mut scylla::serialize::writers::RowWriter) -> Result<(), scylla::serialize::SerializationError> {
        Ok(())
    }
    fn is_empty(&self) -> bool { false }
}

// Implement Repository trait for User
struct UserRepository {
    client: CassandraClient,
}

impl UserRepository {
    fn new(client: CassandraClient) -> Self {
        Self { client }
    }
    
    async fn setup_keyspace(&self) -> cassandra_rust_client::Result<()> {
        // Create keyspace
        let create_keyspace = r#"
            CREATE KEYSPACE IF NOT EXISTS user_db
            WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}
        "#;
        
        self.client.session().query(create_keyspace, &[]).await
            .map_err(|e| cassandra_rust_client::CassandraError::QueryError(e.to_string()))?;
        
        // Use keyspace
        self.client.use_keyspace("user_db").await?;
        
        // Create table
        let create_table = r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                name TEXT,
                email TEXT,
                age INT
            )
        "#;
        
        self.client.session().query(create_table, &[]).await
            .map_err(|e| cassandra_rust_client::CassandraError::QueryError(e.to_string()))?;
        
        Ok(())
    }
}

#[async_trait]
impl Repository<User> for UserRepository {
    async fn insert(&self, user: &User) -> cassandra_rust_client::Result<()> {
        let query = "INSERT INTO users (id, name, email, age) VALUES (?, ?, ?, ?)";
        self.client.execute(query, (&user.id, &user.name, &user.email, user.age)).await
    }
    
    async fn find_by_id(&self, id: &str) -> cassandra_rust_client::Result<Option<User>> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| cassandra_rust_client::CassandraError::SerializationError(e.to_string()))?;
        
        let query = "SELECT id, name, email, age FROM users WHERE id = ?";
        let result: Vec<(Uuid, String, String, i32)> = self.client.query(query, (uuid,)).await?;
        
        Ok(result.into_iter().next().map(|(id, name, email, age)| User {
            id,
            name,
            email,
            age,
        }))
    }
    
    async fn find_all(&self) -> cassandra_rust_client::Result<Vec<User>> {
        let query = "SELECT id, name, email, age FROM users";
        let result: Vec<(Uuid, String, String, i32)> = self.client.query(query, ()).await?;
        
        Ok(result.into_iter().map(|(id, name, email, age)| User {
            id,
            name,
            email,
            age,
        }).collect())
    }
    
    async fn update(&self, user: &User) -> cassandra_rust_client::Result<()> {
        let query = "UPDATE users SET name = ?, email = ?, age = ? WHERE id = ?";
        self.client.execute(query, (&user.name, &user.email, user.age, &user.id)).await
    }
    
    async fn delete(&self, id: &str) -> cassandra_rust_client::Result<()> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| cassandra_rust_client::CassandraError::SerializationError(e.to_string()))?;
        
        let query = "DELETE FROM users WHERE id = ?";
        self.client.execute(query, (uuid,)).await
    }
}

#[tokio::main]
async fn main() -> cassandra_rust_client::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Configure client
    let config = CassandraConfig {
        contact_points: vec!["127.0.0.1:9042".to_string()],
        keyspace: None,
        consistency: ConsistencyLevel::Quorum,
        connection_timeout_ms: 5000,
        request_timeout_ms: 10000,
        connections_per_host: 4,
        compression: true,
        auth: None,
    };
    
    // Create client
    println!("Connecting to Cassandra...");
    let client = CassandraClient::new(config).await?;
    
    // Check health
    match client.health_check().await {
        Ok(healthy) => println!("Health check: {}", if healthy { "OK" } else { "FAILED" }),
        Err(e) => println!("Health check error: {}", e),
    }
    
    // Create repository
    let user_repo = UserRepository::new(client.clone());
    
    // Setup database
    println!("\nSetting up database...");
    user_repo.setup_keyspace().await?;
    
    // Create a new user
    let new_user = User {
        id: Uuid::new_v4(),
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        age: 30,
    };
    
    println!("\nInserting user: {:?}", new_user);
    user_repo.insert(&new_user).await?;
    
    // Find user by ID
    println!("\nFinding user by ID: {}", new_user.id);
    if let Some(user) = user_repo.find_by_id(&new_user.id.to_string()).await? {
        println!("Found user: {:?}", user);
    }
    
    // Update user
    let mut updated_user = new_user.clone();
    updated_user.age = 31;
    println!("\nUpdating user age to 31");
    user_repo.update(&updated_user).await?;
    
    // Find all users
    println!("\nFinding all users:");
    let all_users = user_repo.find_all().await?;
    for user in &all_users {
        println!("  {:?}", user);
    }
    
    // Delete user
    println!("\nDeleting user: {}", new_user.id);
    user_repo.delete(&new_user.id.to_string()).await?;
    
    // Verify deletion
    println!("\nVerifying deletion...");
    let remaining_users = user_repo.find_all().await?;
    println!("Remaining users: {}", remaining_users.len());
    
    // Demonstrate query builder
    let query = QueryBuilder::new()
        .select(&["id", "name", "email"])
        .from("users")
        .where_clause("age > ?")
        .limit(100)
        .build();
    println!("\nGenerated query: {}", query);
    
    // Demonstrate retry policy
    let retry_policy = RetryPolicy::new(3, 100);
    println!("\nRetry policy configured: max_retries=3, delay=100ms");
    
    println!("\n✅ All operations completed successfully!");
    
    Ok(())
}
