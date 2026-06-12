use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapability {
    pub product: String,
    pub config: bool,
    pub production: bool,
    pub mcu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ATResponse {
    pub lines: Vec<String>,
    pub success: bool,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: String,
    pub command: String,
    pub status: String,
    pub raw_response: String,
    pub parsed_data: HashMap<String, String>,
    pub error: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub exist: bool,
    pub size: u32,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUploadResult {
    pub lines_sent: usize,
    pub readback: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub port_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialDataEvent {
    pub line: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub key: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReportItem {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub status: String,
    pub data: HashMap<String, String>,
    pub raw: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub id: String,
    pub timestamp: String,
    pub operator: String,
    pub device: DeviceReportInfo,
    pub overall: String,
    pub duration_ms: u64,
    pub items: Vec<TestReportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceReportInfo {
    pub product: String,
    pub imei: String,
    pub iccid: String,
    pub fw_version: String,
    pub bt_version: String,
    pub bt_mac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub id: String,
    pub timestamp: String,
    pub imei: String,
    pub product: String,
    pub overall: String,
    pub operator: String,
    pub pass_count: usize,
    pub fail_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub result: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItemConfig {
    pub id: String,
    pub enabled: bool,
    pub retries: u32,
    pub timeout_ms: u64,
    pub params: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub operator: String,
    pub baud_rate: u32,
    pub data_dir: String,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default = "default_true")]
    pub keep_production_mode: bool,
    pub test_items: Vec<TestItemConfig>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            operator: String::new(),
            baud_rate: 115200,
            data_dir: String::new(),
            auto_reconnect: true,
            keep_production_mode: true,
            test_items: default_test_items(),
        }
    }
}

pub fn default_test_items() -> Vec<TestItemConfig> {
    vec![
        TestItemConfig {
            id: "MDSIM".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MDREG".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MDSIG".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: [("csq_min".into(), serde_json::json!(10)), ("rssi_min".into(), serde_json::json!(-90)), ("rsrp_min".into(), serde_json::json!(-110))].into(),
        },
        TestItemConfig {
            id: "MDDATA".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MDALL".into(), enabled: true, retries: 1, timeout_ms: 30000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MDPING".into(), enabled: true, retries: 1, timeout_ms: 20000,
            params: [("ping_host".into(), serde_json::json!("8.8.8.8")), ("ping_count".into(), serde_json::json!(3))].into(),
        },
        TestItemConfig {
            id: "MCUBVER".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUMAC".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUCHG".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUVBAT".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: [("mv_min".into(), serde_json::json!(3000)), ("mv_max".into(), serde_json::json!(4500))].into(),
        },
        TestItemConfig {
            id: "MCULED".into(), enabled: true, retries: 0, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUFBMIC".into(), enabled: true, retries: 0, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUPMIC".into(), enabled: true, retries: 0, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUKEY".into(), enabled: true, retries: 0, timeout_ms: 5000,
            params: [("timeout_s".into(), serde_json::json!(30)), ("key_timeout_s".into(), serde_json::json!(10))].into(),
        },
        TestItemConfig {
            id: "MCUGAUGE".into(), enabled: true, retries: 1, timeout_ms: 8000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCUTIME".into(), enabled: true, retries: 1, timeout_ms: 5000,
            params: HashMap::new(),
        },
        TestItemConfig {
            id: "MCURST".into(), enabled: true, retries: 0, timeout_ms: 8000,
            params: HashMap::new(),
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub name: String,
    pub content: String,
    pub description: String,
    pub updated_at: String,
}
