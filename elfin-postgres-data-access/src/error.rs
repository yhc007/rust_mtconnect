use thiserror::Error;

/// 데이터베이스 서비스 관련 에러 타입
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// PostgreSQL 연결 및 쿼리 에러
    #[error("Database connection error: {0}")]
    Connection(#[from] tokio_postgres::Error),

    /// JSON 직렬화/역직렬화 에러
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 유효하지 않은 데이터 에러
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// 환경 변수 관련 에러
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
}

/// Result 타입 별칭: Result<T, DatabaseError>
pub type Result<T> = std::result::Result<T, DatabaseError>;