use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDef {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyTestConfig {
    pub keys: Vec<KeyDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductConfig {
    pub product: String,
    pub key_test: KeyTestConfig,
}

impl Default for ProductConfig {
    fn default() -> Self {
        Self {
            product: "UNKNOWN".to_string(),
            key_test: KeyTestConfig {
                keys: vec![
                    KeyDef {
                        name: "MULTI_FUN".to_string(),
                        label: "多功能键".to_string(),
                        note: "侧面大按键".to_string(),
                    },
                    KeyDef {
                        name: "VOL_UP".to_string(),
                        label: "音量+".to_string(),
                        note: "顶部左键".to_string(),
                    },
                    KeyDef {
                        name: "VOL_DOWN".to_string(),
                        label: "音量-/关机键".to_string(),
                        note: "顶部右键".to_string(),
                    },
                ],
            },
        }
    }
}

fn products_dir() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".esim-production-tool")
        .join("products");
    base
}

pub fn load_product_config(product: &str) -> ProductConfig {
    let path = products_dir().join(format!("{}.json", product));
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str::<ProductConfig>(&content) {
            return config;
        }
    }
    let mut default = ProductConfig::default();
    default.product = product.to_string();
    default
}

pub fn ensure_default_configs() {
    let dir = products_dir();
    let _ = fs::create_dir_all(&dir);

    let e02t_path = dir.join("E02T.json");
    if !e02t_path.exists() {
        let config = ProductConfig {
            product: "E02T".to_string(),
            key_test: KeyTestConfig {
                keys: vec![
                    KeyDef {
                        name: "MULTI_FUN".to_string(),
                        label: "多功能键".to_string(),
                        note: "GPIO14, 侧面大按键, 短按PTT/长按其他".to_string(),
                    },
                    KeyDef {
                        name: "VOL_UP".to_string(),
                        label: "音量+".to_string(),
                        note: "顶部左键".to_string(),
                    },
                    KeyDef {
                        name: "VOL_DOWN".to_string(),
                        label: "音量-/关机键".to_string(),
                        note: "顶部右键, 长按关机".to_string(),
                    },
                ],
            },
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&e02t_path, json);
        }
    }
}
