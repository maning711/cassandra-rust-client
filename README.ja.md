# Cassandra Rust クライアント

Rust で書かれた高パフォーマンス・型安全な Cassandra データベースクライアントライブラリ。

## 特徴

- ✅ **非同期サポート**: Tokio による完全非同期操作
- ✅ **型安全性**: Rust の強力な型システムによるコンパイル時安全保証
- ✅ **接続プール**: 高並行性対応の組み込み接続プール管理
- ✅ **リトライ機構**: 指数バックオフ戦略による自動リトライ
- ✅ **サーキットブレーカー**: カスケード障害を防ぐサーキットブレーカーパターン
- ✅ **クエリビルダー**: 型安全な CQL クエリ構築
- ✅ **リポジトリパターン**: 抽象化されたデータアクセスレイヤー
- ✅ **ゼロコピー**: メモリ割り当てとコピーを最小化
- ✅ **圧縮サポート**: LZ4 圧縮によるネットワーク転送量の削減

## アーキテクチャ

### システムアーキテクチャ図

```mermaid
graph TB
    subgraph "アプリケーション層"
        APP[Application Code]
        REPO_IMPL["UserRepository / OrderRepository"]
    end

    subgraph "リポジトリ層"
        REPO_TRAIT["Repository Trait\ninsert / find / update / delete"]
        QB["QueryBuilder\n型安全 CQL"]
        POOL_MGR["PoolManager"]
    end

    subgraph "クライアント層"
        CLIENT["CassandraClient"]
        SESSION["Arc(Session)\nスレッドセーフ共有"]
        STMT_CACHE["Prepared Statement キャッシュ"]
    end

    subgraph "レジリエンス層"
        RETRY["RetryPolicy\n指数バックオフ"]
        CB["CircuitBreaker\n高速フェイル"]
        HEALTH["ヘルスチェック"]
    end

    subgraph "設定層"
        CONFIG["CassandraConfig\ncontact_points / consistency\ntimeouts / compression"]
        AUTH["AuthConfig\nusername / password"]
    end

    subgraph "トランスポート層"
        SCYLLA["Scylla Driver"]
        CONN_POOL["接続プール\nホストごとの接続"]
        COMPRESS["LZ4 圧縮"]
    end

    subgraph "Cassandra クラスター"
        LB["ロードバランサー"]
        NODE1["ノード 1"]
        NODE2["ノード 2"]
        NODE3["ノード 3"]
    end

    subgraph "エラーハンドリング"
        ERROR["CassandraError\nConnectionError / QueryError\nSerializationError"]
    end

    APP --> REPO_IMPL
    REPO_IMPL --> REPO_TRAIT
    REPO_IMPL --> QB
    REPO_TRAIT --> CLIENT
    QB --> CLIENT
    POOL_MGR --> CLIENT

    CLIENT --> SESSION
    CLIENT --> STMT_CACHE
    CLIENT --> CONFIG
    CONFIG --> AUTH

    SESSION --> RETRY
    RETRY --> CB
    CB --> HEALTH

    RETRY --> SCYLLA
    CB --> SCYLLA
    SCYLLA --> CONN_POOL
    SCYLLA --> COMPRESS

    CONN_POOL --> LB
    LB --> NODE1
    LB --> NODE2
    LB --> NODE3

    CLIENT -. error .-> ERROR
    RETRY -. error .-> ERROR
    CB -. error .-> ERROR
    SCYLLA -. error .-> ERROR

    style APP fill:#dbeafe
    style CLIENT fill:#fef9c3
    style RETRY fill:#fee2e2
    style CB fill:#fee2e2
    style SCYLLA fill:#dcfce7
    style ERROR fill:#fce7f3
```

### クエリ実行シーケンス図

```mermaid
sequenceDiagram
    participant App as アプリケーション
    participant Repo as リポジトリ
    participant Client as CassandraClient
    participant Retry as RetryPolicy
    participant CB as CircuitBreaker
    participant Pool as 接続プール
    participant DB as Cassandra クラスター

    App->>Repo: find_by_id("user-123")
    Repo->>Client: query(cql, params)
    Client->>Client: Prepare Statement (キャッシュ確認)
    Client->>Client: 整合性レベル設定

    Client->>Retry: execute_with_retry()

    loop 最大 3 回リトライ
        Retry->>CB: サーキット状態確認
        alt サーキット閉じている
            CB->>Pool: 接続取得
            Pool->>DB: クエリ実行
            alt 成功
                DB-->>Pool: 結果行
                Pool-->>CB: 成功
                CB->>CB: record_success()
                CB-->>Retry: 結果
            else ネットワークエラー
                DB--xPool: エラー
                CB->>CB: record_failure()
                CB-->>Retry: エラー
                Retry->>Retry: バックオフ (100ms -> 200ms -> 400ms)
            end
        else サーキット開いている
            CB-->>Retry: 高速フェイル
        end
    end

    Retry-->>Client: Result
    Client->>Client: Rust 型へデシリアライズ
    Client-->>Repo: Result[User]
    Repo-->>App: Option[User]
```

### コアモジュール

1. **クライアント層** (`client.rs`)
   - Cassandra クラスターへの接続管理
   - 統一されたクエリ実行インターフェース
   - セッションライフサイクルと接続プール管理

2. **設定層** (`config.rs`)
   - 柔軟な設定管理
   - 複数の整合性レベルのサポート
   - 認証とタイムアウト設定

3. **リポジトリ層** (`repository.rs`)
   - データアクセスパターンの抽象化
   - CRUD 操作インターフェースの提供
   - 型安全なクエリビルダー

4. **リトライ & サーキットブレーカー** (`retry.rs`)
   - 失敗した操作の自動リトライ
   - サーキットブレーカーによるシステム過負荷防止
   - 指数バックオフ戦略

5. **エラーハンドリング** (`error.rs`)
   - 統一されたエラー処理
   - 詳細なエラー分類
   - エラーチェーンのトレース

## クイックスタート

### 依存関係の追加

`Cargo.toml` に追加:

```toml
[dependencies]
cassandra-rust-client = { path = "./cassandra-rust-client" }
tokio = { version = "1.35", features = ["full"] }
```

### 基本的な使い方

```rust
use cassandra_rust_client::{CassandraClient, CassandraConfig, ConsistencyLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // クライアントの設定
    let config = CassandraConfig {
        contact_points: vec!["127.0.0.1:9042".to_string()],
        keyspace: Some("my_keyspace".to_string()),
        consistency: ConsistencyLevel::Quorum,
        ..Default::default()
    };

    // クライアントの作成
    let client = CassandraClient::new(config).await?;

    // クエリの実行
    client.execute(
        "INSERT INTO users (id, name) VALUES (?, ?)",
        (uuid::Uuid::new_v4(), "John Doe")
    ).await?;

    Ok(())
}
```

### リポジトリパターンの使用

```rust
use cassandra_rust_client::{Repository, CassandraClient};
use async_trait::async_trait;

struct User {
    id: Uuid,
    name: String,
    email: String,
}

struct UserRepository {
    client: CassandraClient,
}

#[async_trait]
impl Repository<User> for UserRepository {
    async fn insert(&self, user: &User) -> Result<()> {
        self.client.execute(
            "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
            (&user.id, &user.name, &user.email)
        ).await
    }

    // ... 他のメソッドを実装
}
```

## なぜ Rust なのか？

### 1. パフォーマンス
- **ゼロコスト抽象化**: 抽象化レイヤーによる実行時オーバーヘッドなし
- **GC なし**: ガベージコレクションの停止なし — 予測可能な低レイテンシ
- **メモリ効率**: 精密なメモリ制御で最小限のフットプリント
- **SIMD サポート**: CPU の SIMD 命令を活用した高速化

**パフォーマンス比較** (他言語との比較):
- Python/Ruby より **10〜100 倍** 高速
- Java/C# より **2〜5 倍** 高速
- C/C++ と同等の性能

### 2. メモリ安全性
- **コンパイル時保証**: メモリエラーをコンパイル時に検出
- **ヌルポインタなし**: `Option<T>` でヌルポインタ例外を排除
- **データ競合なし**: 所有権システムで並行バグを防止
- **Use-After-Free なし**: ライフタイムチェックでダングリングポインタを排除

```rust
let data = vec![1, 2, 3];
let reference = &data[0];
// drop(data); // コンパイルエラー！reference はまだ使用中
println!("{}", reference);
```

### 3. 並行安全性
- **Send/Sync トレイト**: スレッド安全性をコンパイル時に検証
- **データ競合なし**: 型システムが並行安全性を保証
- **Async/Await**: 効率的な非同期プログラミングモデル

### 4. 表現力
- **パターンマッチング**: 強力な構造的パターンマッチング
- **代数的データ型**: `Result`/`Option` で明示的なエラー処理を強制
- **トレイトシステム**: 柔軟な多態性とコード再利用
- **マクロシステム**: コンパイル時メタプログラミング

```rust
// エラー処理の強制 — 暗黙的な無視は不可能
match client.query("SELECT * FROM users").await {
    Ok(users) => process_users(users),
    Err(e) => handle_error(e),
}
```

### 5. 言語比較

| 特徴 | Rust | Java | Python | Go |
|------|------|------|--------|-----|
| パフォーマンス | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐ |
| メモリ安全性 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| 並行性 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| 学習曲線 | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| エコシステム | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| コンパイル時チェック | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐ |

## サンプルの実行

```bash
# Cassandra を起動
docker run -d -p 9042:9042 cassandra:latest

# サンプルを実行
cargo run --example basic_usage
```

## テスト

```bash
cargo test
```

## ベンチマーク

```bash
cargo bench
```

## ライセンス

MIT License

## コントリビューション

Issue や Pull Request を歓迎します！
