use serde::{Deserialize, Serialize};

/// CNC 데이터 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CncData {
    #[serde(rename = "shopId")]
    pub shop_id: i32,
    #[serde(rename = "machineId")]
    pub machine_id: String,
    #[serde(rename = "ncId")]
    pub nc_id: Option<String>,
    pub timestamp: Option<i64>,
    #[serde(rename = "partCount")]
    pub part_count: Option<i32>,
    #[serde(rename = "totalPartCount")]
    pub total_part_count: Option<i32>,
    pub mode: Option<String>,
    #[serde(rename = "mainPgmNm")]
    pub main_pgm_nm: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "pathData")]
    pub path_data: Option<Vec<PathDataSet>>,
    pub alarms: Option<Vec<Alarm>>,
}

/// 경로 데이터 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathDataSet {
    pub path: i32,
    #[serde(rename = "spindleLoad")]
    pub spindle_load: f64,
    #[serde(rename = "spindleOverride")]
    pub spindle_override: Option<i32>,
    #[serde(rename = "spindleSpeed")]
    pub spindle_speed: i32,
    #[serde(rename = "feedOverride")]
    pub feed_override: Option<i32>,
    #[serde(rename = "auxCodes")]
    pub aux_codes: Option<AuxCodes>,
}

/// 보조 코드 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuxCodes {
    #[serde(rename = "T")]
    pub t: Option<String>,
}

/// 알람 정보 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    #[serde(rename = "type")]
    pub alarm_type: Option<String>,
    #[serde(rename = "alarmCode")]
    pub alarm_code: Option<String>,
    #[serde(rename = "alarmMessage")]
    pub alarm_message: Option<String>,
}

/// CNC 머신 정보 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CncMachine {
    pub id: i32,
    #[serde(rename = "ncId")]
    pub nc_id: String,
    #[serde(rename = "machineName")]
    pub machine_name: String,
    #[serde(rename = "ncHost")]
    pub nc_host: String,
    #[serde(rename = "ncPort")]
    pub nc_port: i32,
    #[serde(rename = "shopId")]
    pub shop_id: i32,
    pub location: Option<String>,
    pub model: Option<String>,
    pub macros: String,
    #[serde(rename = "cycleTime")]
    pub cycle_time: i32,
    #[serde(rename = "macroCycleTime")]
    pub macro_cycle_time: i32,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// 매크로 데이터 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroData {
    #[serde(rename = "shopId")]
    pub shop_id: i32,
    #[serde(rename = "machineId")]
    pub machine_id: i32,
    pub timestamp: Option<i64>,
    pub macros: Option<serde_json::Value>,
}

/// 머신 상태 구조체 (실시간 상태)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStatus {
    pub id: i64,
    #[serde(rename = "machineId")]
    pub machine_id: i64,
    pub timestamp: i64,
    #[serde(rename = "partCount")]
    pub part_count: Option<i32>,
    #[serde(rename = "totalPartCount")]
    pub total_part_count: Option<i32>,
    pub status: Option<String>,
    pub mode: Option<String>,
    #[serde(rename = "mainProgram")]
    pub main_program: Option<String>,
    #[serde(rename = "spindleLoad")]
    pub spindle_load: Option<f64>,
    #[serde(rename = "spindleOverride")]
    pub spindle_override: Option<i32>,
    #[serde(rename = "spindleSpeed")]
    pub spindle_speed: Option<i32>,
    #[serde(rename = "feedOverride")]
    pub feed_override: Option<i32>,
    #[serde(rename = "auxCodesJson")]
    pub aux_codes_json: Option<String>,
    #[serde(rename = "alarmsJson")]
    pub alarms_json: Option<String>,
    #[serde(rename = "pathDataJson")]
    pub path_data_json: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// 머신 데이터 이력 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineDataHistory {
    pub id: i64,
    #[serde(rename = "machineId")]
    pub machine_id: i64,
    #[serde(rename = "shopId")]
    pub shop_id: i32,
    pub timestamp: i64,
    #[serde(rename = "partCount")]
    pub part_count: Option<i32>,
    #[serde(rename = "totalPartCount")]
    pub total_part_count: Option<i32>,
    pub status: Option<String>,
    pub mode: Option<String>,
    #[serde(rename = "mainProgram")]
    pub main_program: Option<String>,
    #[serde(rename = "spindleLoad")]
    pub spindle_load: Option<f64>,
    #[serde(rename = "spindleOverride")]
    pub spindle_override: Option<i32>,
    #[serde(rename = "spindleSpeed")]
    pub spindle_speed: Option<i32>,
    #[serde(rename = "feedOverride")]
    pub feed_override: Option<i32>,
    #[serde(rename = "auxCodesJson")]
    pub aux_codes_json: Option<String>,
    #[serde(rename = "alarmsJson")]
    pub alarms_json: Option<String>,
    #[serde(rename = "pathDataJson")]
    pub path_data_json: Option<String>,
    #[serde(rename = "rawDataJson")]
    pub raw_data_json: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// 매크로 이력 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineMacroHistory {
    pub id: i64,
    #[serde(rename = "machineId")]
    pub machine_id: i64,
    #[serde(rename = "shopId")]
    pub shop_id: i32,
    pub timestamp: i64,
    #[serde(rename = "macrosJson")]
    pub macros_json: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}