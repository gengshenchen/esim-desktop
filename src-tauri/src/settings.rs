use crate::types::{AppSettings, ConfigTemplate};
use std::fs;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".esim-production-tool")
}

fn settings_path() -> PathBuf {
    data_dir().join("config.json")
}

fn templates_dir() -> PathBuf {
    data_dir().join("templates")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str(&content) {
            return settings;
        }
    }
    let mut settings = AppSettings::default();
    settings.data_dir = data_dir().to_string_lossy().to_string();
    settings
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let _ = fs::create_dir_all(data_dir());
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| format!("序列化设置失败: {}", e))?;
    fs::write(settings_path(), json).map_err(|e| format!("保存设置失败: {}", e))?;
    Ok(())
}

pub fn list_templates() -> Result<Vec<ConfigTemplate>, String> {
    let dir = templates_dir();
    let _ = fs::create_dir_all(&dir);

    let mut templates = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(tpl) = serde_json::from_str::<ConfigTemplate>(&content) {
                    templates.push(tpl);
                }
            }
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

pub fn save_template(template: &ConfigTemplate) -> Result<(), String> {
    let dir = templates_dir();
    let _ = fs::create_dir_all(&dir);

    let filename = format!("{}.json", sanitize_filename(&template.name));
    let path = dir.join(filename);
    let json =
        serde_json::to_string_pretty(template).map_err(|e| format!("序列化模板失败: {}", e))?;
    fs::write(path, json).map_err(|e| format!("保存模板失败: {}", e))?;
    Ok(())
}

pub fn delete_template(name: &str) -> Result<(), String> {
    let dir = templates_dir();
    let filename = format!("{}.json", sanitize_filename(name));
    let path = dir.join(filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("删除模板失败: {}", e))?;
    }
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
