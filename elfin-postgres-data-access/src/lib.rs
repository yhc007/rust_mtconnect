// Rust PostgreSQL Data Access Library
// C# DatabaseService.cs를 Rust로 포팅한 라이브러리

pub mod models;
pub mod error;
pub mod database_service;

// 공개 API re-export
pub use database_service::DatabaseService;
pub use error::{DatabaseError, Result};
pub use models::*;