use anyhow::Result;
use mtconnect_client::MachineDataCollector;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let agent_url = "http://agent.mtconnect.org";
    let mut collector = MachineDataCollector::new(agent_url.to_string());

    // 초기화
    println!("Connecting to {}", agent_url);
    let initial = collector.initialize().await?;
    println!("Connected to: {}", initial.device_name);

    // 10회만 수집 후 종료
    let mut count = 0;
    collector.collect_continuous(Duration::from_secs(2), move |snapshot| {
        count += 1;
        println!("{}: {:?}", count, snapshot.execution);
        
        if count >= 10 {
            std::process::exit(0);
        }
    }).await?;

    Ok(())
}
