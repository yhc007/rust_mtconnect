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

    /// 연결 시도가 제한 시간을 넘김.
    /// 타임아웃이 없으면 tokio_postgres::connect 가 영구히 멈출 수 있다.
    #[error("Database connect timed out after {0:?}")]
    ConnectTimeout(std::time::Duration),

    /// 다른 호출이 이미 재연결 중이다. 기다리지 않고 이번 주기를 넘긴다.
    #[error("Database reconnect already in progress")]
    ReconnectBusy,
}

/// Result 타입 별칭: Result<T, DatabaseError>
pub type Result<T> = std::result::Result<T, DatabaseError>;