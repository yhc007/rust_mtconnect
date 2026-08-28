use chrono::{DateTime, Utc};
use uuid::Uuid;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MTConnectAgent {
    instance_id: String,
    sender: String,
    version: String,
    devices: Vec<MTConnectDevice>,
    sequence: u64,
    remote_agents: Vec<(String, String)>, // (base_url, device_name) 리스트
    client: Arc<Client<HttpConnector, Full<Bytes>>>,
    cached_data: Arc<RwLock<HashMap<String, String>>>, // IP별 캐시된 데이터
}

#[derive(Debug, Clone)]
pub struct MTConnectDevice {
    pub id: String,
    pub name: String,
    pub uuid: String,
    pub data_items: Vec<DataItem>,
}

#[derive(Debug, Clone)]
pub struct DataItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub data_type: String,
    pub value: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl MTConnectAgent {
    pub fn new() -> Self {
        let remote_agents = vec![
            ("http://192.168.10.5:5000".to_string(), "HCN6800".to_string()),
            ("http://192.168.10.5:5001".to_string(), "QT350-1".to_string()),
            ("http://192.168.10.5:5002".to_string(), "QT350-2".to_string()),
            ("http://192.168.10.5:5003".to_string(), "QT350-3".to_string()),
            ("http://192.168.10.5:5004".to_string(), "QT350-4".to_string()),
            // 장비명은 에이전트가 보고하는 uuid의 모델명과 일치시킨다.
            // 5006(MT600_SN306319)과 5007(MT600S_SN306321)이 서로 뒤바뀌어 있었다.
            ("http://192.168.10.5:5005".to_string(), "MNT600-1".to_string()),   // MT600_SN306318
            ("http://192.168.10.5:5006".to_string(), "MNT600-2".to_string()),   // MT600_SN306319
            ("http://192.168.10.5:5007".to_string(), "MNT600S-1".to_string()),  // MT600S_SN306321
            ("http://192.168.10.5:5008".to_string(), "MNT600S-2".to_string()),  // MT600S_SN306322
        ];

        let client = Arc::new(
            Client::builder(TokioExecutor::new())
                .build_http()
        );

        let mut agent = MTConnectAgent {
            instance_id: Uuid::new_v4().to_string(),
            sender: "MTConnect Rust Server".to_string(),
            version: "1.8".to_string(),
            devices: Vec::new(),
            sequence: 1,
            remote_agents,
            client,
            cached_data: Arc::new(RwLock::new(HashMap::new())),
        };

        let device = MTConnectDevice::new(
            "device1".to_string(),
            "CNC Machine".to_string(),
        );
        agent.devices.push(device);
        agent
    }

    pub fn get_probe_response(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<MTConnectDevices xmlns=\"urn:mtconnect.org:MTConnectDevices:1.8\">\n");
        xml.push_str("  <Header creationTime=\"");
        xml.push_str(&Utc::now().to_rfc3339());
        xml.push_str("\" sender=\"");
        xml.push_str(&self.sender);
        xml.push_str("\" instanceId=\"");
        xml.push_str(&self.instance_id);
        xml.push_str("\" version=\"");
        xml.push_str(&self.version);
        xml.push_str("\"/>\n");
        xml.push_str("  <Devices>\n");
        
        for device in &self.devices {
            xml.push_str("    <Device id=\"");
            xml.push_str(&device.id);
            xml.push_str("\" name=\"");
            xml.push_str(&device.name);
            xml.push_str("\" uuid=\"");
            xml.push_str(&device.uuid);
            xml.push_str("\">\n");
            xml.push_str("      <DataItems>\n");
            
            for data_item in &device.data_items {
                xml.push_str("        <DataItem id=\"");
                xml.push_str(&data_item.id);
                xml.push_str("\" name=\"");
                xml.push_str(&data_item.name);
                xml.push_str("\" category=\"");
                xml.push_str(&data_item.category);
                xml.push_str("\" type=\"");
                xml.push_str(&data_item.data_type);
                xml.push_str("\"/>\n");
            }
            
            xml.push_str("      </DataItems>\n");
            xml.push_str("    </Device>\n");
        }
        
        xml.push_str("  </Devices>\n");
        xml.push_str("</MTConnectDevices>");
        xml
    }

    // 모든 원격 Agent에서 probe 데이터 가져오기
    pub async fn fetch_probe_from_all(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<MTConnectDevices xmlns=\"urn:mtconnect.org:MTConnectDevices:1.8\">\n");
        xml.push_str("  <Header creationTime=\"");
        xml.push_str(&Utc::now().to_rfc3339());
        xml.push_str("\" sender=\"");
        xml.push_str(&self.sender);
        xml.push_str("\" instanceId=\"");
        xml.push_str(&self.instance_id);
        xml.push_str("\" version=\"");
        xml.push_str(&self.version);
        xml.push_str("\"/>\n");
        xml.push_str("  <Devices>\n");

        // 모든 에이전트에서 병렬로 데이터 가져오기
        let mut tasks = Vec::new();
        for (base_url, device_name) in &self.remote_agents {
            let client = Arc::clone(&self.client);
            let base_url_clone = base_url.clone();
            let device_name_clone = device_name.clone();
            tasks.push(tokio::spawn(async move {
                let url = format!("{}/probe", base_url_clone);
                match url.parse::<hyper::Uri>() {
                    Ok(uri) => {
                        let req = hyper::Request::builder()
                            .uri(uri)
                            .method(hyper::Method::GET)
                            .body(Full::<Bytes>::default());

                        match req {
                            Ok(request) => {
                                match client.request(request).await {
                                    Ok(res) => {
                                        if res.status().is_success() {
                                            let body = res.into_body();
                                            match body.collect().await {
                                                Ok(collected) => {
                                                    let bytes: Bytes = collected.to_bytes();
                                                    match String::from_utf8(bytes.to_vec()) {
                                                        Ok(content) => Some((device_name_clone, content)),
                                                        Err(_) => None,
                                                    }
                                                }
                                                Err(_) => None,
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    Err(_) => None,
                                }
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }));
        }

        // 결과 수집
        for task in tasks {
            if let Ok(Some((device_name, content))) = task.await {
                // XML에서 <Devices>...</Devices> 내용 추출하여 추가
                if let Some(devices_start) = content.find("<Devices>") {
                    if let Some(devices_end) = content.find("</Devices>") {
                        let devices_content = &content[devices_start + 9..devices_end];
                        xml.push_str(&format!("    <!-- Data from {} -->\n", device_name));
                        xml.push_str(devices_content);
                    }
                }
            }
        }

        xml.push_str("  </Devices>\n");
        xml.push_str("</MTConnectDevices>");
        xml
    }

    pub fn get_current_response(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<MTConnectStreams xmlns=\"urn:mtconnect.org:MTConnectStreams:1.8\">\n");
        xml.push_str("  <Header creationTime=\"");
        xml.push_str(&Utc::now().to_rfc3339());
        xml.push_str("\" sender=\"");
        xml.push_str(&self.sender);
        xml.push_str("\" instanceId=\"");
        xml.push_str(&self.instance_id);
        xml.push_str("\" version=\"");
        xml.push_str(&self.version);
        xml.push_str("\" firstSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" lastSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" nextSequence=\"");
        xml.push_str(&(self.sequence + 1).to_string());
        xml.push_str("\"/>\n");
        xml.push_str("  <Streams>\n");
        
        for device in &self.devices {
            xml.push_str("    <DeviceStream name=\"");
            xml.push_str(&device.name);
            xml.push_str("\" uuid=\"");
            xml.push_str(&device.uuid);
            xml.push_str("\">\n");
            xml.push_str("      <ComponentStream component=\"Device\" name=\"");
            xml.push_str(&device.name);
            xml.push_str("\">\n");
            
            for data_item in &device.data_items {
                if let Some(value) = &data_item.value {
                    xml.push_str("        <");
                    xml.push_str(&data_item.data_type);
                    xml.push_str(" dataItemId=\"");
                    xml.push_str(&data_item.id);
                    xml.push_str("\" timestamp=\"");
                    xml.push_str(&data_item.timestamp.to_rfc3339());
                    xml.push_str("\" sequence=\"");
                    xml.push_str(&self.sequence.to_string());
                    xml.push_str("\">");
                    xml.push_str(value);
                    xml.push_str("</");
                    xml.push_str(&data_item.data_type);
                    xml.push_str(">\n");
                }
            }
            
            xml.push_str("      </ComponentStream>\n");
            xml.push_str("    </DeviceStream>\n");
        }
        
        xml.push_str("  </Streams>\n");
        xml.push_str("</MTConnectStreams>");
        xml
    }

    // 모든 원격 Agent에서 current 데이터 가져오기
    pub async fn fetch_current_from_all(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<MTConnectStreams xmlns=\"urn:mtconnect.org:MTConnectStreams:1.8\">\n");
        xml.push_str("  <Header creationTime=\"");
        xml.push_str(&Utc::now().to_rfc3339());
        xml.push_str("\" sender=\"");
        xml.push_str(&self.sender);
        xml.push_str("\" instanceId=\"");
        xml.push_str(&self.instance_id);
        xml.push_str("\" version=\"");
        xml.push_str(&self.version);
        xml.push_str("\" firstSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" lastSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" nextSequence=\"");
        xml.push_str(&(self.sequence + 1).to_string());
        xml.push_str("\"/>\n");
        xml.push_str("  <Streams>\n");

        // 모든 에이전트에서 병렬로 데이터 가져오기
        let mut tasks = Vec::new();
        for (base_url, device_name) in &self.remote_agents {
            let client = Arc::clone(&self.client);
            let base_url_clone = base_url.clone();
            let device_name_clone = device_name.clone();
            tasks.push(tokio::spawn(async move {
                let url = format!("{}/current", base_url_clone);
                match url.parse::<hyper::Uri>() {
                    Ok(uri) => {
                        let req = hyper::Request::builder()
                            .uri(uri)
                            .method(hyper::Method::GET)
                            .body(Full::<Bytes>::default());

                        match req {
                            Ok(request) => {
                                match client.request(request).await {
                                    Ok(res) => {
                                        if res.status().is_success() {
                                            let body = res.into_body();
                                            match body.collect().await {
                                                Ok(collected) => {
                                                    let bytes: Bytes = collected.to_bytes();
                                                    match String::from_utf8(bytes.to_vec()) {
                                                        Ok(content) => Some((device_name_clone, content)),
                                                        Err(_) => None,
                                                    }
                                                }
                                                Err(_) => None,
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    Err(_) => None,
                                }
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }));
        }

        // 결과 수집
        for task in tasks {
            if let Ok(Some((device_name, content))) = task.await {
                xml.push_str(&format!("    <!-- Data from {} -->\n", device_name));
                // XML에서 <Streams>...</Streams> 내용 추출하여 추가
                if let Some(streams_start) = content.find("<Streams>") {
                    if let Some(streams_end) = content.find("</Streams>") {
                        let streams_content = &content[streams_start + 9..streams_end];
                        xml.push_str(streams_content);
                    }
                }
            }
        }

        xml.push_str("  </Streams>\n");
        xml.push_str("</MTConnectStreams>");
        xml
    }

    pub fn get_sample_response(&self) -> String {
        self.get_current_response()
    }

    /// 모든 원격 Agent에서 current 데이터 가져오고 파싱된 DeviceInfo도 반환
    pub async fn fetch_current_with_devices(&self) -> (String, Vec<DeviceInfo>) {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<MTConnectStreams xmlns=\"urn:mtconnect.org:MTConnectStreams:1.8\">\n");
        xml.push_str("  <Header creationTime=\"");
        xml.push_str(&Utc::now().to_rfc3339());
        xml.push_str("\" sender=\"");
        xml.push_str(&self.sender);
        xml.push_str("\" instanceId=\"");
        xml.push_str(&self.instance_id);
        xml.push_str("\" version=\"");
        xml.push_str(&self.version);
        xml.push_str("\" firstSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" lastSequence=\"");
        xml.push_str(&self.sequence.to_string());
        xml.push_str("\" nextSequence=\"");
        xml.push_str(&(self.sequence + 1).to_string());
        xml.push_str("\"/>\n");
        xml.push_str("  <Streams>\n");

        let mut all_devices: Vec<DeviceInfo> = Vec::new();

        // 모든 에이전트에서 병렬로 데이터 가져오기
        let mut tasks = Vec::new();
        for (base_url, device_name) in &self.remote_agents {
            let client = Arc::clone(&self.client);
            let base_url_clone = base_url.clone();
            let device_name_clone = device_name.clone();
            tasks.push(tokio::spawn(async move {
                let url = format!("{}/current", base_url_clone);
                match url.parse::<hyper::Uri>() {
                    Ok(uri) => {
                        let req = hyper::Request::builder()
                            .uri(uri)
                            .method(hyper::Method::GET)
                            .body(Full::<Bytes>::default());

                        match req {
                            Ok(request) => {
                                // 5초 타임아웃 적용
                                match tokio::time::timeout(
                                    Duration::from_secs(5),
                                    client.request(request)
                                ).await {
                                    Ok(Ok(res)) => {
                                        if res.status().is_success() {
                                            let body = res.into_body();
                                            match body.collect().await {
                                                Ok(collected) => {
                                                    let bytes: Bytes = collected.to_bytes();
                                                    match String::from_utf8(bytes.to_vec()) {
                                                        Ok(content) => Some((device_name_clone, content)),
                                                        Err(_) => None,
                                                    }
                                                }
                                                Err(_) => None,
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None, // 타임아웃 또는 에러
                                }
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                }
            }));
        }

        // 결과 수집
        for task in tasks {
            if let Ok(Some((device_name, content))) = task.await {
                // DeviceInfo 파싱
                let devices = parse_device_info(&content, &device_name);
                all_devices.extend(devices);

                xml.push_str(&format!("    <!-- Data from {} -->\n", device_name));
                // XML에서 <Streams>...</Streams> 내용 추출하여 추가
                if let Some(streams_start) = content.find("<Streams>") {
                    if let Some(streams_end) = content.find("</Streams>") {
                        let streams_content = &content[streams_start + 9..streams_end];
                        xml.push_str(streams_content);
                    }
                }
            }
        }

        xml.push_str("  </Streams>\n");
        xml.push_str("</MTConnectStreams>");

        (xml, all_devices)
    }

    /// 원격 에이전트 목록 반환 (base_url, device_name)
    pub fn get_remote_agents(&self) -> &[(String, String)] {
        &self.remote_agents
    }
}

impl MTConnectDevice {
    pub fn new(id: String, name: String) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let mut device = MTConnectDevice {
            id,
            name,
            uuid,
            data_items: Vec::new(),
        };

        device.data_items.push(DataItem {
            id: "avail".to_string(),
            name: "avail".to_string(),
            category: "EVENT".to_string(),
            data_type: "Availability".to_string(),
            value: Some("AVAILABLE".to_string()),
            timestamp: Utc::now(),
        });

        device.data_items.push(DataItem {
            id: "estop".to_string(),
            name: "estop".to_string(),
            category: "EVENT".to_string(),
            data_type: "EmergencyStop".to_string(),
            value: Some("ARMED".to_string()),
            timestamp: Utc::now(),
        });

        device.data_items.push(DataItem {
            id: "mode".to_string(),
            name: "mode".to_string(),
            category: "EVENT".to_string(),
            data_type: "ControllerMode".to_string(),
            value: Some("AUTOMATIC".to_string()),
            timestamp: Utc::now(),
        });

        device
    }
}

// ============================================================================
// Device Info for Database Storage
// ============================================================================

use elfin_postgres_data_access::{AuxSignals, CncData, PathDataSet};

/// 수집된 디바이스 정보 구조체
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub ip: String,
    pub name: String,
    pub uuid: String,
    pub availability: String,
    pub execution: String,
    pub controller_mode: String,
    pub emergency_stop: String,
    pub program_name: String,
    pub subprogram_name: String,
    pub part_count: String,
    pub spindle_load: String,
    pub spindle_rpm: String,
    pub spindle_override: String,
    pub feed_override: String,
    pub rapid_override: String,
    pub feedrate_actual: String,
    pub x_axis_load: String,
    pub z_axis_load: String,
    pub pallet_num: String,
    pub line_num: String,
    pub auto_time: String,
    pub cut_time: String,
    pub total_time: String,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        DeviceInfo {
            ip: String::new(),
            name: "Unknown".to_string(),
            uuid: "N/A".to_string(),
            availability: "UNAVAILABLE".to_string(),
            execution: "N/A".to_string(),
            controller_mode: "N/A".to_string(),
            emergency_stop: "N/A".to_string(),
            program_name: "N/A".to_string(),
            subprogram_name: "N/A".to_string(),
            part_count: "0".to_string(),
            spindle_load: "0".to_string(),
            spindle_rpm: "0".to_string(),
            spindle_override: "100".to_string(),
            feed_override: "100".to_string(),
            rapid_override: "100".to_string(),
            feedrate_actual: "0".to_string(),
            x_axis_load: "0".to_string(),
            z_axis_load: "0".to_string(),
            pallet_num: "N/A".to_string(),
            line_num: "N/A".to_string(),
            auto_time: "N/A".to_string(),
            cut_time: "N/A".to_string(),
            total_time: "N/A".to_string(),
        }
    }
}

impl DeviceInfo {
    /// DeviceInfo를 CncData로 변환
    pub fn to_cnc_data(&self, machine_id: &str, shop_id: i32) -> CncData {
        let spindle_load = self.spindle_load.parse::<f64>().unwrap_or(0.0);
        let spindle_override = self.spindle_override.parse::<i32>().ok();
        // 주축 회전수는 RotaryVelocity(Srpm)에서 수집한다.
        // 에이전트가 소수(예: "1500.0")나 UNAVAILABLE을 보낼 수 있으므로 f64로 파싱 후 변환
        let spindle_speed = self.spindle_rpm.parse::<f64>().map(|v| v as i32).unwrap_or(0);
        let feed_override = self.feed_override.parse::<i32>().ok();
        // UNAVAILABLE/누락 시 0이 아닌 None(NULL)으로 저장하여 "0개 가공"과 "미측정"을 구분
        let part_count = self.part_count.parse::<i32>().ok();

        // 실제 가동 중인 프로그램명.
        // 메인 프로그램(Program name="program")은 QT-MAIN/HCN-MAIN 같은 고정 이름이라
        // 가공 중인 프로그램 추적에 쓸 수 없다. 실시간으로 갱신되는 서브프로그램
        // (Program name="subprogram", subType="x:SUB")을 우선 사용하고,
        // 값이 없을 때만 메인 프로그램으로 폴백한다.
        let running_pgm = if is_usable(&self.subprogram_name) {
            Some(self.subprogram_name.clone())
        } else if self.program_name != "N/A" {
            Some(self.program_name.clone())
        } else {
            None
        };

        // 값이 없는 신호는 None으로 남겨 "미수집"과 "0"을 구분한다
        // ACCUMULATED_TIME은 SAMPLE이라 소수로 올 수 있어 f64로 받고 초 단위로 절삭한다
        let secs = |v: &str| {
            if is_usable(v) { v.parse::<f64>().ok().map(|n| n as i64) } else { None }
        };
        let text = |v: &str| if is_usable(v) { Some(v.to_string()) } else { None };
        let aux_signals = AuxSignals {
            total_time: secs(&self.total_time),
            auto_time: secs(&self.auto_time),
            cut_time: secs(&self.cut_time),
            pallet_num: text(&self.pallet_num),
            line_num: text(&self.line_num),
            subprogram: text(&self.subprogram_name),
        };

        CncData {
            shop_id,
            machine_id: machine_id.to_string(),
            nc_id: Some(machine_id.to_string()),
            timestamp: Some(Utc::now().timestamp_millis()),
            part_count,
            // 이 에이전트들은 누적 파트카운트를 제공하지 않는다.
            // /probe 상 PART_COUNT 타입 DataItem은 pc(PartCountAct) 하나뿐이므로
            // part_count를 복사하지 않고 NULL로 남겨 "누적값 없음"을 명시한다.
            total_part_count: None,
            mode: Some(self.controller_mode.clone()),
            main_pgm_nm: running_pgm,
            status: Some(self.execution.clone()),
            path_data: Some(vec![PathDataSet {
                path: 1,
                spindle_load,
                spindle_override,
                spindle_speed,
                feed_override,
                aux_codes: None,
            }]),
            alarms: None,
            aux_signals: Some(aux_signals),
        }
    }
}

/// XML에서 속성 값 추출
fn extract_attribute(text: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = text.find(&pattern)?;
    let start_pos = start + pattern.len();
    let end = text[start_pos..].find("\"")?;
    Some(text[start_pos..start_pos + end].to_string())
}

/// XML에서 특정 name 속성을 가진 태그의 값 추출
fn extract_value_by_name(xml: &str, tag_name: &str, name_attr: &str) -> String {
    let search_pattern = format!("name=\"{}\"", name_attr);

    let Some(name_pos) = xml.find(&search_pattern) else {
        return "N/A".to_string();
    };
    let Some(tag_start) = xml[..name_pos].rfind(&format!("<{} ", tag_name)) else {
        return "N/A".to_string();
    };

    // 여는 태그의 끝('>')을 먼저 찾는다
    let Some(open_end_rel) = xml[tag_start..].find('>') else {
        return "N/A".to_string();
    };
    let open_end = tag_start + open_end_rel;

    // self-closing 태그(<Program ... />)는 값이 비어 있다는 뜻이다.
    // 이를 걸러내지 않으면 닫는 태그를 찾지 못해 다음 엘리먼트까지 XML 마크업이 통째로 반환된다.
    if xml[tag_start..open_end].ends_with('/') {
        return "N/A".to_string();
    }

    let closing_tag = format!("</{}>", tag_name);
    let Some(close_rel) = xml[open_end + 1..].find(&closing_tag) else {
        return "N/A".to_string();
    };

    xml[open_end + 1..open_end + 1 + close_rel].trim().to_string()
}

/// 수집값이 실제 의미 있는 값인지 판정 (미수집/미가용과 구분)
fn is_usable(value: &str) -> bool {
    !value.is_empty() && value != "N/A" && value != "UNAVAILABLE"
}

/// XML에서 DeviceInfo 파싱
pub fn parse_device_info(xml: &str, ip: &str) -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    let mut search_pos = 0;
    while let Some(device_start) = xml[search_pos..].find("<DeviceStream") {
        let abs_start = search_pos + device_start;
        if let Some(device_end) = xml[abs_start..].find("</DeviceStream>") {
            let device_xml = &xml[abs_start..abs_start + device_end + 15];

            let mut device = DeviceInfo::default();
            device.ip = ip.to_string();

            // DeviceStream 태그에서 name, uuid 추출
            if let Some(name_end) = device_xml.find(">") {
                let header = &device_xml[..name_end];
                if let Some(name) = extract_attribute(header, "name") {
                    device.name = name;
                }
                if let Some(uuid) = extract_attribute(header, "uuid") {
                    device.uuid = uuid;
                }
            }

            // 데이터 항목 추출
            device.availability = extract_value_by_name(device_xml, "Availability", "avail");
            device.execution = extract_value_by_name(device_xml, "Execution", "execution");
            device.controller_mode = extract_value_by_name(device_xml, "ControllerMode", "mode");
            device.emergency_stop = extract_value_by_name(device_xml, "EmergencyStop", "estop");
            device.program_name = extract_value_by_name(device_xml, "Program", "program");
            device.subprogram_name = extract_value_by_name(device_xml, "Program", "subprogram");
            device.part_count = extract_value_by_name(device_xml, "PartCount", "PartCountAct");
            device.spindle_load = extract_value_by_name(device_xml, "Load", "Sload");
            device.spindle_rpm = extract_value_by_name(device_xml, "RotaryVelocity", "Srpm");
            device.spindle_override = extract_value_by_name(device_xml, "RotaryVelocityOverride", "Sovr");
            device.feed_override = extract_value_by_name(device_xml, "PathFeedrateOverride", "Fovr");
            device.rapid_override = extract_value_by_name(device_xml, "PathFeedrateOverride", "Frapidovr");
            device.feedrate_actual = extract_value_by_name(device_xml, "PathFeedrate", "Fact");
            device.x_axis_load = extract_value_by_name(device_xml, "Load", "Xload");
            device.z_axis_load = extract_value_by_name(device_xml, "Load", "Zload");
            // 파트카운트가 죽어 있어도 살아 있는 누적 카운터와 사이클 지표.
            // 전용 컬럼이 없어 raw_data(jsonb)에만 저장한다.
            device.pallet_num = extract_value_by_name(device_xml, "PalletId", "pallet_num");
            device.line_num = extract_value_by_name(device_xml, "Line", "line");
            device.auto_time = extract_value_by_name(device_xml, "AccumulatedTime", "auto_time");
            device.cut_time = extract_value_by_name(device_xml, "AccumulatedTime", "cut_time");
            device.total_time = extract_value_by_name(device_xml, "AccumulatedTime", "total_time");

            devices.push(device);
            search_pos = abs_start + device_end + 15;
        } else {
            break;
        }
    }

    devices
}