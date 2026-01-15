use chrono::{DateTime, Utc};
use uuid::Uuid;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MTConnectAgent {
    instance_id: String,
    sender: String,
    version: String,
    devices: Vec<MTConnectDevice>,
    sequence: u64,
    remote_agents: Vec<String>, // IP 주소 리스트
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
            "192.168.20.10".to_string(),
            "192.168.20.20".to_string(),
            "192.168.20.30".to_string(),
            "192.168.20.40".to_string(),
            "192.168.20.50".to_string(),
            "192.168.20.60".to_string(),
            "192.168.20.70".to_string(),
            "192.168.20.80".to_string(),
            "192.168.20.100".to_string(),
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

        // 모든 IP에서 병렬로 데이터 가져오기
        let mut tasks = Vec::new();
        for ip in &self.remote_agents {
            let client = Arc::clone(&self.client);
            let ip_clone = ip.clone();
            tasks.push(tokio::spawn(async move {
                let url = format!("http://{}:5000/probe", ip_clone);
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
                                                        Ok(content) => Some((ip_clone, content)),
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
            if let Ok(Some((ip, content))) = task.await {
                // XML에서 <Devices>...</Devices> 내용 추출하여 추가
                if let Some(devices_start) = content.find("<Devices>") {
                    if let Some(devices_end) = content.find("</Devices>") {
                        let devices_content = &content[devices_start + 9..devices_end];
                        xml.push_str(&format!("    <!-- Data from {} -->\n", ip));
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

        // 모든 IP에서 병렬로 데이터 가져오기
        let mut tasks = Vec::new();
        for ip in &self.remote_agents {
            let client = Arc::clone(&self.client);
            let ip_clone = ip.clone();
            tasks.push(tokio::spawn(async move {
                let url = format!("http://{}:5000/current", ip_clone);
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
                                                        Ok(content) => Some((ip_clone, content)),
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
            if let Ok(Some((ip, content))) = task.await {
                xml.push_str(&format!("    <!-- Data from {} -->\n", ip));
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