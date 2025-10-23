use anyhow::Result;
use mtconnect_client::{MachineDataCollector, ExecutionState, DataLogger};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // MTConnect Agent URL (예시)
    let agent_url = std::env::var("MTCONNECT_URL")
        .unwrap_or_else(|_| "http://agent.mtconnect.org".to_string());
    
    println!("MTConnect Client Starting...");
    println!("Agent URL: {}", agent_url);
    
    let mut collector = MachineDataCollector::new(agent_url);

    // 데이터 로거 초기화 (옵션)
    let logger = DataLogger::new("machine_data.csv".to_string());
    let _ = logger.log_csv_header().await;

    // 초기화
    println!("\nInitializing collector...");
    match collector.initialize().await {
        Ok(initial) => {
            println!("✓ Connected to device: {}", initial.device_name);
            println!("✓ Initial sequence: {}", initial.sequence);
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize: {}", e);
            return Err(e);
        }
    }

    // 연속 수집 시작
    println!("\n=== Starting Continuous Collection ===");
    println!("Press Ctrl+C to stop\n");
    
    collector.collect_continuous(Duration::from_secs(1), move |snapshot| {
        // 콘솔 출력
        println!("[{}] {}", 
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S"), 
            snapshot.device_name
        );
        
        // 상태 정보
        if let Some(exec) = &snapshot.execution {
            let status_icon = match exec {
                ExecutionState::ACTIVE => "🟢",
                ExecutionState::READY => "🟡",
                ExecutionState::STOPPED => "🔴",
                _ => "⚪",
            };
            println!("  {} Status: {:?}", status_icon, exec);
            
            // 가공중일 때만 상세 정보 출력
            if *exec == ExecutionState::ACTIVE {
                if let Some(prog) = &snapshot.program_name {
                    println!("  📄 Program: {}", prog);
                }
                if let Some(load) = snapshot.spindle_load {
                    println!("  ⚙️  Spindle Load: {:.1}%", load);
                }
                if let Some(speed) = snapshot.spindle_speed {
                    println!("  🔄 Spindle Speed: {:.0} RPM", speed);
                }
                if let Some(feed) = snapshot.feed_override {
                    println!("  ➡️  Feed Override: {:.0}%", feed);
                }
                if let Some(feedrate) = snapshot.feedrate {
                    println!("  📊 Feedrate: {:.1} mm/min", feedrate);
                }
            }
        }
        
        // 생산 개수
        if let Some(count) = snapshot.part_count {
            println!("  📦 Parts Produced: {}", count);
        }
        
        // 알람 체크
        if let Some(alarm) = &snapshot.alarm {
            if !alarm.is_empty() && alarm != "UNAVAILABLE" {
                println!("  ⚠️  ALARM: {}", alarm);
            }
        }
        
        // 비상정지 체크
        if let Some(estop) = &snapshot.emergency_stop {
            if *estop == mtconnect_client::EmergencyState::TRIGGERED {
                println!("  🚨 EMERGENCY STOP TRIGGERED!");
            }
        }
        
        println!();
        
        // 데이터 로깅 (백그라운드)
        let logger_clone = logger.clone();
        let snapshot_clone = snapshot.clone();
        tokio::spawn(async move {
            if let Err(e) = logger_clone.log(&snapshot_clone).await {
                eprintln!("Failed to log data: {}", e);
            }
        });
    }).await?;

    Ok(())
}
