use elfin_postgres_data_access::*;
use chrono::Utc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 초기화
    tracing_subscriber::fmt::init();

    // 데이터베이스 서비스 생성 (환경 변수에서 설정 읽기)
    let db = DatabaseService::from_env().await?;
    println!("✅ Database service initialized");

    // 연결 테스트
    db.test_connection().await?;
    println!("✅ Database connection test successful");

    // 활성 머신 조회
    let machines = db.get_active_machines().await?;
    println!("✅ Found {} active machines", machines.len());

    for machine in &machines {
        println!("  - {} ({}) at {}:{}", machine.machine_name, machine.nc_id, machine.nc_host, machine.nc_port);
    }

    // CNC 데이터 저장 예시
    if !machines.is_empty() {
        let machine = &machines[0];

        // 샘플 CNC 데이터 생성
        let cnc_data = CncData {
            shop_id: machine.shop_id,
            machine_id: machine.id.to_string(),
            nc_id: Some(machine.nc_id.clone()),
            timestamp: Some(Utc::now().timestamp_millis()),
            part_count: Some(15),
            total_part_count: Some(21758),
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
            // FOCAS 수집기는 보조 신호를 쓰지 않으므로 None
            aux_signals: None,
        };

        // 실시간 상태 업데이트
        db.update_machine_status(&cnc_data).await?;
        println!("✅ Machine status updated");

        // 이력 데이터 저장
        db.save_machine_data_history(&cnc_data).await?;
        println!("✅ Machine data history saved");

        // 매크로 데이터 저장 예시
        let macro_data = MacroData {
            shop_id: machine.shop_id,
            machine_id: machine.id,
            timestamp: Some(Utc::now().timestamp_millis()),
            macros: Some(serde_json::json!({"macro1": 100, "macro2": 200})),
        };

        db.save_macro_history(&macro_data).await?;
        println!("✅ Macro history saved");
    }

    println!("🎉 All operations completed successfully!");
    Ok(())
}