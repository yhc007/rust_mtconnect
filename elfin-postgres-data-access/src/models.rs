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
    /// 전용 컬럼이 없는 보조 신호. machine_data_history.raw_data(jsonb)에만 실린다.
    /// None이면 직렬화에서 빠지므로 기존 수집기가 만드는 JSON은 그대로 유지된다.
    #[serde(rename = "auxSignals", skip_serializing_if = "Option::is_none", default)]
    pub aux_signals: Option<AuxSignals>,
}

/// 전용 컬럼 없이 raw_data에 보관하는 보조 신호.
///
/// MTConnect 에이전트는 파트카운트가 죽어 있어도 누적 시간 카운터와
/// 가공 사이클 지표는 정상적으로 내보낸다. 스키마를 바꾸지 않고
/// 이후 분석에 쓰기 위해 여기에 모아 둔다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuxSignals {
    /// 누적 총 시간(초)
    #[serde(rename = "totalTime", skip_serializing_if = "Option::is_none", default)]
    pub total_time: Option<i64>,
    /// 누적 자동운전 시간(초)
    #[serde(rename = "autoTime", skip_serializing_if = "Option::is_none", default)]
    pub auto_time: Option<i64>,
    /// 누적 절삭 시간(초)
    #[serde(rename = "cutTime", skip_serializing_if = "Option::is_none", default)]
    pub cut_time: Option<i64>,
    /// 팔레트 번호 (팔레트 교환식 장비의 파트 교체 지표)
    #[serde(rename = "palletNum", skip_serializing_if = "Option::is_none", default)]
    pub pallet_num: Option<String>,
    /// 실행 중인 블록 번호
    #[serde(rename = "lineNum", skip_serializing_if = "Option::is_none", default)]
    pub line_num: Option<String>,
    /// 서브프로그램명 (가공 중에만 값이 존재)
    #[serde(rename = "subprogram", skip_serializing_if = "Option::is_none", default)]
    pub subprogram: Option<String>,
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