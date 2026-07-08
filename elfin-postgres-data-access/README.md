# Elfin PostgreSQL Data Access Library

C# `DatabaseService.cs`를 Rust로 포팅한 PostgreSQL 데이터 액세스 라이브러리입니다. CNC 공작기계 데이터를 PostgreSQL 데이터베이스에 저장하고 조회하는 비동기 라이브러리입니다.

## 특징

- **비동기 처리**: `tokio` 기반의 비동기 PostgreSQL 클라이언트
- **타입 안전성**: Rust의 강력한 타입 시스템 활용
- **JSON 지원**: PostgreSQL JSONB 컬럼 지원
- **에러 처리**: 명시적인 `Result<T, E>` 기반 에러 처리
- **로깅**: `tracing` 기반 구조화된 로깅

## 지원 기능

- ✅ CNC 머신 정보 조회
- ✅ 실시간 상태 데이터 UPSERT 저장
- ✅ 이력 데이터 저장
- ✅ 매크로 데이터 저장
- ✅ 연결 테스트
- ✅ 환경 변수 기반 설정

## 설치

### Cargo.toml에 추가

```toml
[dependencies]
elfin-postgres-data-access = { path = "../elfin-postgres-data-access" }
```

### 환경 변수 설정

```bash
# 필수 환경 변수
export DB_HOST="localhost"
export DB_PORT="5432"
export DB_NAME="focas_db"
export DB_USER="focas_user"
export DB_PASSWORD="your_password"
```

## 사용 방법

### 기본 사용법

```rust
use elfin_postgres_data_access::*;
use chrono::Utc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 초기화
    tracing_subscriber::fmt::init();

    // 데이터베이스 서비스 생성
    let db = DatabaseService::from_env().await?;

    // 연결 테스트
    db.test_connection().await?;

    // 활성 머신 조회
    let machines = db.get_active_machines().await?;
    println!("Found {} active machines", machines.len());

    Ok(())
}
```

### CNC 데이터 저장

```rust
use elfin_postgres_data_access::{DatabaseService, CncData, PathDataSet, Alarm};

let db = DatabaseService::from_env().await?;

// CNC 데이터 생성
let cnc_data = CncData {
    shop_id: 1,
    machine_id: 123456,
    nc_id: Some("CM2002".to_string()),
    timestamp: Some(Utc::now().timestamp_millis()),
    part_count: 15,
    total_part_count: 21758,
    mode: Some("HANDLE".to_string()),
    main_pgm_nm: Some("O2331".to_string()),
    status: Some("RESET".to_string()),
    path_data: Some(vec![PathDataSet {
        path: 1,
        spindle_load: 0.0,
        spindle_override: Some(100),
        spindle_speed: 0,
        feed_override: Some(155),
        aux_codes: Some(AuxCodes {
            t: Some("1200".to_string()),
        }),
    }]),
    alarms: Some(vec![Alarm {
        alarm_type: Some("PMC".to_string()),
        alarm_code: Some("2001".to_string()),
        alarm_message: Some("알람 메시지".to_string()),
    }]),
};

// 실시간 상태 저장 (UPSERT)
db.update_machine_status(&cnc_data).await?;

// 이력 데이터 저장
db.save_machine_data_history(&cnc_data).await?;
```

### 직접 연결 문자열 사용

```rust
let connection_string = "host=localhost port=5432 dbname=focas_db user=focas_user password=password";
let db = DatabaseService::new(connection_string).await?;
```

## API 문서

### DatabaseService

#### 생성자

- `new(connection_string: &str) -> Result<Self>`: 연결 문자열로 생성
- `from_env() -> Result<Self>`: 환경 변수에서 설정 읽기

#### 메서드

- `get_active_machines() -> Result<Vec<CncMachine>>`: 활성 CNC 머신 목록 조회
- `update_machine_status(data: &CncData) -> Result<()>`: 실시간 상태 UPSERT 저장
- `save_machine_data_history(data: &CncData) -> Result<()>`: 이력 데이터 저장
- `save_macro_history(macro_data: &MacroData) -> Result<()>`: 매크로 데이터 저장
- `test_connection() -> Result<()>`: 데이터베이스 연결 테스트

## 데이터 모델

### CncData
CNC 공작기계의 실시간 데이터 구조체

```rust
pub struct CncData {
    pub shop_id: i32,
    pub machine_id: i32,
    pub nc_id: Option<String>,
    pub timestamp: Option<i64>,
    pub part_count: i32,
    pub total_part_count: i32,
    pub mode: Option<String>,
    pub main_pgm_nm: Option<String>,
    pub status: Option<String>,
    pub path_data: Option<Vec<PathDataSet>>,
    pub alarms: Option<Vec<Alarm>>,
}
```

### CncMachine
CNC 머신 정보 구조체

```rust
pub struct CncMachine {
    pub id: i32,
    pub nc_id: String,
    pub machine_name: String,
    pub nc_host: String,
    pub nc_port: i32,
    pub shop_id: i32,
    pub location: Option<String>,
    pub model: Option<String>,
    pub macros: String,
    pub cycle_time: i32,
    pub macro_cycle_time: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## 데이터베이스 스키마

### cnc_machines 테이블
```sql
CREATE TABLE cnc_machines (
    id VARCHAR PRIMARY KEY,
    nc_id VARCHAR,
    machine_name VARCHAR,
    nc_host VARCHAR,
    nc_port INTEGER DEFAULT 8193,
    shop_id INTEGER DEFAULT 1,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### machine_status 테이블
```sql
CREATE TABLE machine_status (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    machine_id VARCHAR NOT NULL,
    nc_id VARCHAR(100),
    shop_id INTEGER,
    timestamp BIGINT,
    part_count INTEGER DEFAULT 0,
    total_part_count INTEGER DEFAULT 0,
    status VARCHAR,
    mode VARCHAR,
    main_pgm_nm VARCHAR,
    spindle_load DOUBLE PRECISION DEFAULT 0,
    spindle_override INTEGER DEFAULT 0,
    spindle_speed DOUBLE PRECISION DEFAULT 0,
    feed_override INTEGER DEFAULT 0,
    aux_codes JSONB,
    alarms JSONB,
    path_data JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (machine_id)
);
```

### machine_data_history 테이블
```sql
CREATE TABLE machine_data_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    machine_id VARCHAR NOT NULL,
    nc_id VARCHAR(100),
    shop_id INTEGER,
    timestamp BIGINT,
    part_count INTEGER DEFAULT 0,
    total_part_count INTEGER DEFAULT 0,
    status VARCHAR,
    mode VARCHAR,
    main_pgm_nm VARCHAR,
    spindle_load DOUBLE PRECISION DEFAULT 0,
    spindle_override INTEGER DEFAULT 0,
    spindle_speed DOUBLE PRECISION DEFAULT 0,
    feed_override INTEGER DEFAULT 0,
    aux_codes JSONB,
    alarms JSONB,
    path_data JSONB,
    raw_data JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## 예시 실행

```bash
# 환경 변수 설정
export DB_HOST="localhost"
export DB_PORT="5432"
export DB_NAME="focas_db"
export DB_USER="focas_user"
export DB_PASSWORD="your_password"

# 예시 코드 실행
cargo run --example usage
```

## 에러 처리

모든 메서드는 `Result<T, DatabaseError>`를 반환합니다:

```rust
use elfin_postgres_data_access::{DatabaseError, Result};

match db.get_active_machines().await {
    Ok(machines) => println!("Found {} machines", machines.len()),
    Err(DatabaseError::Connection(e)) => eprintln!("Connection error: {}", e),
    Err(DatabaseError::Serialization(e)) => eprintln!("Serialization error: {}", e),
    Err(DatabaseError::InvalidData(msg)) => eprintln!("Invalid data: {}", msg),
}
```

## 로깅

`tracing`을 사용하여 구조화된 로깅을 지원합니다:

```rust
use tracing::{info, error};

info!("Database service initialized");
error!("Failed to save data: {}", e);
```

## 라이선스

이 프로젝트는 MIT 라이선스를 따릅니다.

## 개발

### 빌드

```bash
cargo build
```

### 테스트

```bash
cargo test
```

### 문서 생성

```bash
cargo doc --open
```

## 기여

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request