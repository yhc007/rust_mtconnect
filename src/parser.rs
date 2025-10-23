use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MTConnectStreams {
    #[serde(rename = "Header")]
    pub header: Header,
    #[serde(rename = "Streams")]
    pub streams: Streams,
}

#[derive(Debug, Deserialize)]
pub struct Header {
    #[serde(rename = "@nextSequence")]
    pub next_sequence: u64,
    #[serde(rename = "@instanceId")]
    pub instance_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Streams {
    #[serde(rename = "DeviceStream")]
    pub device_stream: Vec<DeviceStream>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceStream {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "ComponentStream")]
    pub component_stream: Vec<ComponentStream>,
}

#[derive(Debug, Deserialize)]
pub struct ComponentStream {
    #[serde(rename = "@component")]
    pub component: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "Samples", default)]
    pub samples: Option<SampleContainer>,
    #[serde(rename = "Events", default)]
    pub events: Option<EventContainer>,
}

#[derive(Debug, Deserialize)]
pub struct SampleContainer {
    #[serde(rename = "$value")]
    pub items: Vec<SampleItem>,
}

#[derive(Debug, Deserialize)]
pub struct EventContainer {
    #[serde(rename = "$value")]
    pub items: Vec<EventItem>,
}

// Sample 타입 (연속 측정값)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SampleItem {
    Load(DataValue),
    #[serde(rename = "Feedrate")]
    FeedRate(DataValue),
    SpindleSpeed(DataValue),
    Position(DataValue),
}

// Event 타입 (이산 이벤트)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventItem {
    Execution(DataValue),
    ControllerMode(DataValue),
    Program(DataValue),
    PartCount(DataValue),
    #[serde(rename = "PathFeedRateOverride")]
    PathFeedRateOverride(DataValue),
    EmergencyStop(DataValue),
    Message(DataValue),
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataValue {
    #[serde(rename = "@dataItemId")]
    pub data_item_id: String,
    #[serde(rename = "@timestamp")]
    pub timestamp: String,
    #[serde(rename = "@sequence")]
    pub sequence: u64,
    #[serde(rename = "$text", default)]
    pub value: String,
}
