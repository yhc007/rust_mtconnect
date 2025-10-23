use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 공작기계에서 수집할 주요 데이터
#[derive(Debug, Clone, Serialize)]
pub struct MachineSnapshot {
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub device_name: String,
    
    // 스핀들 관련
    pub spindle_load: Option<f64>,          // % (0-100)
    pub spindle_speed: Option<f64>,         // RPM
    pub spindle_override: Option<f64>,      // % (0-200)
    
    // 이송 관련
    pub feed_override: Option<f64>,         // % (0-200)
    pub feedrate: Option<f64>,              // mm/min
    
    // 생산 관련
    pub part_count: Option<u32>,            // 생산 개수
    pub program_name: Option<String>,       // 가공 프로그램명
    
    // 상태 관련
    pub execution: Option<ExecutionState>,  // 실행 상태
    pub controller_mode: Option<ControllerMode>, // 제어 모드
    pub emergency_stop: Option<EmergencyState>,  // 비상정지
    pub alarm: Option<String>,              // 알람 메시지
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ExecutionState {
    READY,           // 준비
    ACTIVE,          // 가공중
    INTERRUPTED,     // 중단
    STOPPED,         // 정지
    PROGRAM_STOPPED, // 프로그램 정지
    OPTIONAL_STOP,   // 옵셔널 스톱
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ControllerMode {
    AUTOMATIC,       // 자동 모드
    MANUAL,          // 수동 모드
    MDI,            // MDI 모드
    EDIT,           // 편집 모드
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum EmergencyState {
    ARMED,          // 정상
    TRIGGERED,      // 비상정지 작동
}
