use crate::machine_data::MachineSnapshot;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use std::io::Result;

#[derive(Clone)]
pub struct DataLogger {
    file_path: String,
}

impl DataLogger {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }

    pub async fn log(&self, snapshot: &MachineSnapshot) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await?;

        let line = format!(
            "{},{},{},{:?},{},{},{},{}\n",
            snapshot.timestamp.to_rfc3339(),
            snapshot.device_name,
            snapshot.program_name.as_deref().unwrap_or(""),
            snapshot.execution.as_ref(),
            snapshot.spindle_load.unwrap_or(0.0),
            snapshot.feed_override.unwrap_or(0.0),
            snapshot.part_count.unwrap_or(0),
            snapshot.alarm.as_deref().unwrap_or("")
        );

        file.write_all(line.as_bytes()).await?;
        Ok(())
    }

    pub async fn log_csv_header(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&self.file_path)
            .await?;

        let header = "timestamp,device_name,program_name,execution,spindle_load,feed_override,part_count,alarm\n";
        file.write_all(header.as_bytes()).await?;
        Ok(())
    }
}
