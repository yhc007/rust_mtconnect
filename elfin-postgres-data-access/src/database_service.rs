use crate::error::{DatabaseError, Result};
use crate::models::{CncData, CncMachine, MacroData};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, NoTls, Row};

/// PostgreSQL 데이터베이스 서비스
/// 연결 시도 제한 시간. tokio_postgres::connect 자체에는 타임아웃이 없어
/// 서버가 소켓만 받고 핸드셰이크를 끝내지 않으면 영구히 멈춘다.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct DatabaseService {
    client: RwLock<Arc<Client>>,
    /// 재연결에 쓰기 위해 보관한다
    connection_string: String,
    /// 재연결 시도를 직렬화한다. try_lock 으로만 잡아 대기를 만들지 않는다.
    reconnect_lock: Mutex<()>,
}

impl DatabaseService {
    /// 커넥션을 맺고 구동용 태스크를 띄운다.
    ///
    /// tokio_postgres::connect 는 자체 타임아웃이 없다. 서버가 소켓만 받고
    /// 핸드셰이크를 끝내지 않으면 영구히 멈추므로 반드시 감싼다.
    async fn connect(connection_string: &str) -> Result<Arc<Client>> {
        let (client, connection) = match tokio::time::timeout(
            CONNECT_TIMEOUT,
            tokio_postgres::connect(connection_string, NoTls),
        )
        .await
        {
            Ok(result) => result.map_err(DatabaseError::Connection)?,
            Err(_) => return Err(DatabaseError::ConnectTimeout(CONNECT_TIMEOUT)),
        };

        // 연결을 백그라운드에서 실행
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Arc::new(client))
    }

    /// 연결 문자열로 생성
    pub async fn new(connection_string: &str) -> Result<Self> {
        let client = Self::connect(connection_string).await?;

        Ok(Self {
            client: RwLock::new(client),
            connection_string: connection_string.to_string(),
            reconnect_lock: Mutex::new(()),
        })
    }

    /// 끊긴 커넥션을 새로 맺는다.
    ///
    /// 서버가 커넥션을 닫으면(디스크 풀로 인한 PANIC 등) tokio-postgres는 스스로
    /// 복구하지 않는다. 재연결하지 않으면 프로세스를 재시작할 때까지 모든 쿼리가
    /// 영구히 Closed로 실패한다.
    async fn reconnect(&self) -> Result<Arc<Client>> {
        // 재연결은 한 번에 하나만 시도한다. 단 기다리지는 않는다 —
        // 10대가 같은 주기에 실패하면 9개는 즉시 에러로 돌아가고 3초 뒤 다시 온다.
        let _attempt = match self.reconnect_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(DatabaseError::ReconnectBusy),
        };

        // 락을 잡는 사이 이미 복구됐으면 그것을 쓴다
        {
            let current = self.client.read().await.clone();
            if !current.is_closed() {
                return Ok(current);
            }
        }

        // 연결은 어떤 client 락도 들지 않은 채 시도한다 (타임아웃은 connect 안에 있다).
        // 이전 구현은 쓰기 락을 든 상태로 connect 를 await 해서, connect 가 멈추면
        // 락이 영구히 잡히고 tokio RwLock 의 공정 큐 때문에 모든 읽기까지 막혀
        // 프로세스 전체가 교착됐다 (2026-10-31, 12일간 수집 중단).
        let client = Self::connect(&self.connection_string).await?;

        // 교체만 짧게. 이 블록 안에서는 await 하지 않는다.
        *self.client.write().await = client.clone();

        tracing::warn!("Database connection was closed; reconnected");
        Ok(client)
    }

    /// 살아 있는 커넥션을 돌려준다. 이미 끊겨 있으면 먼저 다시 맺는다.
    async fn live_client(&self) -> Result<Arc<Client>> {
        let client = { self.client.read().await.clone() };
        if client.is_closed() {
            return self.reconnect().await;
        }
        Ok(client)
    }

    /// execute. 수행 도중 커넥션이 끊기면 한 번만 다시 맺고 재시도한다.
    ///
    /// 커넥션이 살아 있는 상태에서 난 오류는 SQL/제약 조건 문제이므로
    /// 재시도하지 않고 그대로 올린다.
    async fn execute_retry(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.live_client().await?;
        match client.execute(sql, params).await {
            Ok(n) => Ok(n),
            Err(e) => {
                if !client.is_closed() {
                    return Err(DatabaseError::Connection(e));
                }
                let client = self.reconnect().await?;
                client
                    .execute(sql, params)
                    .await
                    .map_err(DatabaseError::Connection)
            }
        }
    }

    /// query. 재시도 규칙은 execute_retry와 같다.
    async fn query_retry(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.live_client().await?;
        match client.query(sql, params).await {
            Ok(rows) => Ok(rows),
            Err(e) => {
                if !client.is_closed() {
                    return Err(DatabaseError::Connection(e));
                }
                let client = self.reconnect().await?;
                client
                    .query(sql, params)
                    .await
                    .map_err(DatabaseError::Connection)
            }
        }
    }

    /// query_one. 재시도 규칙은 execute_retry와 같다.
    async fn query_one_retry(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.live_client().await?;
        match client.query_one(sql, params).await {
            Ok(row) => Ok(row),
            Err(e) => {
                if !client.is_closed() {
                    return Err(DatabaseError::Connection(e));
                }
                let client = self.reconnect().await?;
                client
                    .query_one(sql, params)
                    .await
                    .map_err(DatabaseError::Connection)
            }
        }
    }

    /// 환경 변수에서 설정 읽기
    pub async fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
        let database = std::env::var("DB_NAME").unwrap_or_else(|_| "focas_db".to_string());
        let user = std::env::var("DB_USER").unwrap_or_else(|_| "focas_user".to_string());
        let password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "".to_string());

        let connection_string = format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, database, user, password
        );

        Self::new(&connection_string).await
    }

    /// 활성화된 모든 CNC 설비 정보 조회
    pub async fn get_active_machines(&self) -> Result<Vec<CncMachine>> {
        let rows = self
            .query_retry(
                "SELECT id, nc_id, machine_name, nc_host, nc_port, shop_id, \
                 is_active, created_at, updated_at \
                 FROM cnc_machines \
                 WHERE is_active = true \
                 ORDER BY machine_name",
                &[],
            )
            .await?;

        let mut machines = Vec::new();
        for row in rows {
            let id_str: String = row.get(0);
            let id = id_str.hash_code();

            machines.push(CncMachine {
                id,
                nc_id: row.get::<_, Option<String>>(1).unwrap_or_default(),
                machine_name: row.get::<_, Option<String>>(2).unwrap_or_default(),
                nc_host: row.get::<_, Option<String>>(3).unwrap_or_default(),
                nc_port: row.get::<_, Option<i32>>(4).unwrap_or(8193),
                shop_id: row.get::<_, Option<i32>>(5).unwrap_or(1),
                location: None,
                model: None,
                macros: "Tm9uZQ==".to_string(),
                cycle_time: 4,
                macro_cycle_time: 4,
                is_active: row.get::<_, Option<bool>>(6).unwrap_or(true),
                created_at: row.get::<_, String>(7),
                updated_at: row.get::<_, String>(8),
            });
        }

        tracing::info!("Loaded {} active CNC machines from database", machines.len());
        Ok(machines)
    }

    /// 설비 실시간 상태 업데이트 (UPSERT)
    pub async fn update_machine_status(&self, data: &CncData) -> Result<()> {
        let path_data = data.path_data.as_ref().and_then(|v| v.first());

        let aux_codes_json = path_data
            .and_then(|p| p.aux_codes.as_ref())
            .map(|a| serde_json::to_value(a))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        let alarms_json = data
            .alarms
            .as_ref()
            .map(|a| serde_json::to_value(a))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        let path_data_json = data
            .path_data
            .as_ref()
            .map(|p| serde_json::to_value(p))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        self
            .execute_retry(
                "INSERT INTO machine_status (
                    machine_id, nc_id, shop_id, timestamp, part_count, total_part_count,
                    status, mode, main_pgm_nm, spindle_load, spindle_override,
                    spindle_speed, feed_override, aux_codes, alarms, path_data
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                ON CONFLICT (nc_id)
                DO UPDATE SET
                    nc_id = EXCLUDED.nc_id,
                    shop_id = EXCLUDED.shop_id,
                    timestamp = EXCLUDED.timestamp,
                    part_count = EXCLUDED.part_count,
                    total_part_count = EXCLUDED.total_part_count,
                    status = EXCLUDED.status,
                    mode = EXCLUDED.mode,
                    main_pgm_nm = EXCLUDED.main_pgm_nm,
                    spindle_load = EXCLUDED.spindle_load,
                    spindle_override = EXCLUDED.spindle_override,
                    spindle_speed = EXCLUDED.spindle_speed,
                    feed_override = EXCLUDED.feed_override,
                    aux_codes = EXCLUDED.aux_codes,
                    alarms = EXCLUDED.alarms,
                    path_data = EXCLUDED.path_data,
                    updated_at = CURRENT_TIMESTAMP",
                &[
                    &data.machine_id,
                    &data.nc_id,
                    &(data.shop_id as i32),
                    &data.timestamp,
                    &data.part_count,
                    &data.total_part_count,
                    &data.status,
                    &data.mode,
                    &data.main_pgm_nm,
                    &path_data.map(|p| p.spindle_load).unwrap_or(0.0),
                    &path_data.and_then(|p| p.spindle_override),
                    &(path_data.map(|p| p.spindle_speed as f64).unwrap_or(0.0)),
                    &path_data.and_then(|p| p.feed_override),
                    &aux_codes_json,
                    &alarms_json,
                    &path_data_json,
                ],
            )
            .await?;

        Ok(())
    }

    /// 설비 데이터 이력 저장
    pub async fn save_machine_data_history(&self, data: &CncData) -> Result<()> {
        let path_data = data.path_data.as_ref().and_then(|v| v.first());

        let aux_codes_json = path_data
            .and_then(|p| p.aux_codes.as_ref())
            .map(|a| serde_json::to_value(a))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        let alarms_json = data
            .alarms
            .as_ref()
            .map(|a| serde_json::to_value(a))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        let path_data_json = data
            .path_data
            .as_ref()
            .map(|p| serde_json::to_value(p))
            .transpose()
            .map_err(DatabaseError::Serialization)?;

        let raw_data_json = serde_json::to_value(data)
            .map_err(DatabaseError::Serialization)?;

        self
            .execute_retry(
                "INSERT INTO machine_data_history (
                    machine_id, nc_id, shop_id, timestamp, part_count, total_part_count,
                    status, mode, main_pgm_nm, spindle_load, spindle_override,
                    spindle_speed, feed_override, aux_codes, alarms, path_data, raw_data
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
                &[
                    &data.machine_id,
                    &data.nc_id,
                    &(data.shop_id as i32),
                    &data.timestamp,
                    &data.part_count,
                    &data.total_part_count,
                    &data.status,
                    &data.mode,
                    &data.main_pgm_nm,
                    &path_data.map(|p| p.spindle_load).unwrap_or(0.0),
                    &path_data.and_then(|p| p.spindle_override),
                    &(path_data.map(|p| p.spindle_speed as f64).unwrap_or(0.0)),
                    &path_data.and_then(|p| p.feed_override),
                    &aux_codes_json,
                    &alarms_json,
                    &path_data_json,
                    &raw_data_json,
                ],
            )
            .await?;

        Ok(())
    }

    /// 매크로 데이터 이력 저장
    pub async fn save_macro_history(&self, macro_data: &MacroData) -> Result<()> {
        let macros_json = serde_json::to_value(&macro_data.macros)
            .map_err(DatabaseError::Serialization)?;

        self
            .execute_retry(
                "INSERT INTO machine_macro_history (
                    machine_id, shop_id, timestamp, macros, created_at
                )
                VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
                &[
                    &(macro_data.machine_id as i64),
                    &(macro_data.shop_id as i32),
                    &macro_data.timestamp,
                    &macros_json,
                ],
            )
            .await?;

        Ok(())
    }

    /// 데이터베이스 연결 테스트
    pub async fn test_connection(&self) -> Result<()> {
        self.query_one_retry("SELECT 1", &[]).await?;

        tracing::info!("Database connection successful!");
        Ok(())
    }
}

// String의 해시코드 계산 헬퍼 트레이트
trait HashCode {
    fn hash_code(&self) -> i32;
}

impl HashCode for String {
    fn hash_code(&self) -> i32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        (hasher.finish() as i32).abs()
    }
}