use crate::error::{DatabaseError, Result};
use crate::models::{CncData, CncMachine, MacroData};
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

/// PostgreSQL 데이터베이스 서비스
pub struct DatabaseService {
    client: Arc<Client>,
}

impl DatabaseService {
    /// 연결 문자열로 생성
    pub async fn new(connection_string: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
            .await
            .map_err(DatabaseError::Connection)?;

        // 연결을 백그라운드에서 실행
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self {
            client: Arc::new(client),
        })
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
            .client
            .query(
                "SELECT id, nc_id, machine_name, nc_host, nc_port, shop_id, \
                 is_active, created_at, updated_at \
                 FROM cnc_machines \
                 WHERE is_active = true \
                 ORDER BY machine_name",
                &[],
            )
            .await
            .map_err(DatabaseError::Connection)?;

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

        self.client
            .execute(
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
            .await
            .map_err(DatabaseError::Connection)?;

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

        self.client
            .execute(
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
            .await
            .map_err(DatabaseError::Connection)?;

        Ok(())
    }

    /// 매크로 데이터 이력 저장
    pub async fn save_macro_history(&self, macro_data: &MacroData) -> Result<()> {
        let macros_json = serde_json::to_value(&macro_data.macros)
            .map_err(DatabaseError::Serialization)?;

        self.client
            .execute(
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
            .await
            .map_err(DatabaseError::Connection)?;

        Ok(())
    }

    /// 데이터베이스 연결 테스트
    pub async fn test_connection(&self) -> Result<()> {
        self.client
            .query_one("SELECT 1", &[])
            .await
            .map_err(DatabaseError::Connection)?;

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