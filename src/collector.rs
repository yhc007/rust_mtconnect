use crate::machine_data::*;
use crate::parser::*;
use anyhow::Result;
use quick_xml::de::from_str;
use tokio::time::{sleep, Duration};

pub struct MachineDataCollector {
    client: reqwest::Client,
    base_url: String,
    last_sequence: u64,
}

impl MachineDataCollector {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            last_sequence: 0,
        }
    }

    /// 초기화 - current로 최신 시퀀스 획득
    pub async fn initialize(&mut self) -> Result<MachineSnapshot> {
        let url = format!("{}/current", self.base_url);
        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        let streams: MTConnectStreams = from_str(&text)?;
        
        self.last_sequence = streams.header.next_sequence;
        let snapshot = self.parse_streams(&streams)?;
        
        Ok(snapshot)
    }

    /// 연속 수집 (폴링)
    pub async fn collect_continuous<F>(&mut self, interval: Duration, mut callback: F) -> Result<()>
    where
        F: FnMut(MachineSnapshot),
    {
        loop {
            match self.collect_once().await {
                Ok(Some(snapshot)) => {
                    callback(snapshot);
                }
                Ok(None) => {
                    // 새 데이터 없음
                }
                Err(e) => {
                    eprintln!("Collection error: {}", e);
                    sleep(Duration::from_secs(5)).await; // 에러 시 재시도 대기
                }
            }
            
            sleep(interval).await;
        }
    }

    /// 한 번 수집
    pub async fn collect_once(&mut self) -> Result<Option<MachineSnapshot>> {
        let url = format!("{}/sample?from={}&count=1000", self.base_url, self.last_sequence);
        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        
        let streams: MTConnectStreams = from_str(&text)?;
        
        // 시퀀스가 변하지 않았으면 새 데이터 없음
        if streams.header.next_sequence == self.last_sequence {
            return Ok(None);
        }
        
        self.last_sequence = streams.header.next_sequence;
        let snapshot = self.parse_streams(&streams)?;
        
        Ok(Some(snapshot))
    }

    /// MTConnect 응답을 MachineSnapshot으로 변환
    fn parse_streams(&self, streams: &MTConnectStreams) -> Result<MachineSnapshot> {
        let device_stream = streams.streams.device_stream.get(0)
            .ok_or_else(|| anyhow::anyhow!("No device stream found"))?;

        let mut snapshot = MachineSnapshot {
            timestamp: chrono::Utc::now(),
            sequence: streams.header.next_sequence,
            device_name: device_stream.name.clone(),
            spindle_load: None,
            spindle_speed: None,
            spindle_override: None,
            feed_override: None,
            feedrate: None,
            part_count: None,
            program_name: None,
            execution: None,
            controller_mode: None,
            emergency_stop: None,
            alarm: None,
        };

        // 모든 컴포넌트를 순회하며 데이터 추출
        for comp_stream in &device_stream.component_stream {
            // Samples 처리
            if let Some(samples) = &comp_stream.samples {
                for item in &samples.items {
                    match item {
                        SampleItem::Load(v) if comp_stream.name.contains("Spindle") => {
                            snapshot.spindle_load = v.value.parse().ok();
                        }
                        SampleItem::SpindleSpeed(v) => {
                            snapshot.spindle_speed = v.value.parse().ok();
                        }
                        SampleItem::FeedRate(v) => {
                            snapshot.feedrate = v.value.parse().ok();
                        }
                        _ => {}
                    }
                }
            }

            // Events 처리
            if let Some(events) = &comp_stream.events {
                for item in &events.items {
                    match item {
                        EventItem::Execution(v) => {
                            snapshot.execution = serde_json::from_str(&format!("\"{}\"", v.value)).ok();
                        }
                        EventItem::ControllerMode(v) => {
                            snapshot.controller_mode = serde_json::from_str(&format!("\"{}\"", v.value)).ok();
                        }
                        EventItem::Program(v) => {
                            snapshot.program_name = Some(v.value.clone());
                        }
                        EventItem::PartCount(v) => {
                            snapshot.part_count = v.value.parse().ok();
                        }
                        EventItem::PathFeedRateOverride(v) => {
                            snapshot.feed_override = v.value.parse().ok();
                        }
                        EventItem::EmergencyStop(v) => {
                            snapshot.emergency_stop = serde_json::from_str(&format!("\"{}\"", v.value)).ok();
                        }
                        EventItem::Message(v) => {
                            snapshot.alarm = Some(v.value.clone());
                        }
                    }
                }
            }
        }

        Ok(snapshot)
    }
}
