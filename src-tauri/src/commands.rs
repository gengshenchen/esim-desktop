use crate::at_protocol::{
    get_at_command, get_timeout_ms, judge_test, parse_cap_response, parse_kv_response,
};
use crate::product_config;
use crate::report;
use crate::serial_manager::SharedSerialManager;
use crate::settings;
use crate::types::*;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn scan_ports(serial: State<SharedSerialManager>) -> Vec<String> {
    let mgr = serial.lock().unwrap();
    mgr.scan_ports()
}

#[tauri::command]
pub async fn connect(
    serial: State<'_, SharedSerialManager>,
    port: String,
    app_handle: AppHandle,
) -> Result<DeviceCapability, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        mgr.set_app_handle(app_handle);
        mgr.connect(&port)
            .map_err(|e| format!("打开串口失败: {}", e))?;

        std::thread::sleep(std::time::Duration::from_millis(200));

        let resp = mgr
            .send_command("AT+CAP?", get_timeout_ms("CAP"))
            .map_err(|e| {
                mgr.disconnect();
                format!("串口已打开但设备未响应 AT+CAP? ({})", e)
            })?;

        let cap = parse_cap_response(&resp).map_err(|e| {
            mgr.disconnect();
            format!("设备响应格式异常: {}", e)
        })?;

        Ok(cap)
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn disconnect(serial: State<'_, SharedSerialManager>) -> Result<(), String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        if mgr.is_connected() {
            let _ = mgr.send_command("AT+PROD=0", get_timeout_ms("PROD"));
        }
        mgr.disconnect();
        Ok(())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn send_at_command(
    serial: State<'_, SharedSerialManager>,
    cmd: String,
    timeout_ms: u64,
) -> Result<ATResponse, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        mgr.send_command(&cmd, timeout_ms)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn enter_production_mode(
    serial: State<'_, SharedSerialManager>,
) -> Result<ATResponse, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let max_retries = 3;
        let mut last_err = String::new();

        for attempt in 0..max_retries {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }

            let mut mgr = serial.lock().unwrap();
            match mgr.send_command("AT+PROD=1", get_timeout_ms("PROD")) {
                Ok(resp) if resp.success => {
                    drop(mgr);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    return Ok(resp);
                }
                Ok(resp) => {
                    last_err = resp
                        .error_code
                        .unwrap_or_else(|| "未知错误".to_string());
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }

        Err(format!(
            "进入产测模式失败 (重试{}次): {}",
            max_retries, last_err
        ))
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn exit_production_mode(
    serial: State<'_, SharedSerialManager>,
) -> Result<ATResponse, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+PROD=0", get_timeout_ms("PROD"))
            .map_err(|e| e.to_string())?;

        if !resp.success {
            return Err(format!(
                "退出产测模式失败: {}",
                resp.error_code.unwrap_or_else(|| "未知错误".to_string())
            ));
        }
        Ok(resp)
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn run_single_test(
    serial: State<'_, SharedSerialManager>,
    test_id: String,
) -> Result<TestResult, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let app_settings = settings::load_settings();
        let item_config = app_settings.test_items.iter().find(|i| i.id == test_id);
        let params = item_config.map(|c| &c.params).cloned().unwrap_or_default();
        let timeout =
            item_config.map(|c| c.timeout_ms).unwrap_or_else(|| get_timeout_ms(&test_id));
        let retries = item_config.map(|c| c.retries).unwrap_or(1);

        let cmd = if test_id == "MCUTIME" {
            let now = chrono::Local::now();
            format!("AT+MCUTIME=\"{}\"", now.format("%Y-%m-%d %H:%M:%S"))
        } else if test_id == "MDPING" {
            let host = params.get("ping_host").and_then(|v| v.as_str()).unwrap_or("8.8.8.8");
            let count = params.get("ping_count").and_then(|v| v.as_i64()).unwrap_or(3);
            format!("AT+MDPING=\"{}\",{}", host, count)
        } else {
            let static_cmd = get_at_command(&test_id);
            if static_cmd.is_empty() {
                return Err(format!("未知测试项: {}", test_id));
            }
            static_cmd.to_string()
        };

        let mut last_result = None;
        let attempts = 1 + retries;

        for attempt in 0..attempts {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            let mut mgr = serial.lock().unwrap();
            let start = std::time::Instant::now();
            match mgr.send_command(&cmd, timeout) {
                Ok(resp) => {
                    let duration = start.elapsed().as_millis() as u64;
                    let (status, error) = judge_test(&test_id, &resp, &params);
                    let parsed_data = resp
                        .lines
                        .first()
                        .map(|l| parse_kv_response(l))
                        .unwrap_or_default();

                    let result = TestResult {
                        id: test_id.clone(),
                        command: cmd.clone(),
                        status: status.clone(),
                        raw_response: resp.lines.join("\n"),
                        parsed_data,
                        error,
                        duration_ms: duration,
                    };

                    if status == "pass" || status == "manual_pending" {
                        return Ok(result);
                    }
                    last_result = Some(result);
                }
                Err(e) => {
                    last_result = Some(TestResult {
                        id: test_id.clone(),
                        command: cmd.clone(),
                        status: "fail".to_string(),
                        raw_response: String::new(),
                        parsed_data: HashMap::new(),
                        error: e.to_string(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        Ok(last_result.unwrap())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn run_auto_test(
    serial: State<'_, SharedSerialManager>,
    test_ids: Vec<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let app_settings = settings::load_settings();
        let mut mgr = serial.lock().unwrap();

        for test_id in &test_ids {
            let item_config = app_settings.test_items.iter().find(|i| i.id == *test_id);
            let params = item_config.map(|c| c.params.clone()).unwrap_or_default();
            let timeout =
                item_config.map(|c| c.timeout_ms).unwrap_or_else(|| get_timeout_ms(test_id));
            let retries = item_config.map(|c| c.retries).unwrap_or(1);

            let cmd = if test_id == "MCUTIME" {
                let now = chrono::Local::now();
                format!("AT+MCUTIME=\"{}\"", now.format("%Y-%m-%d %H:%M:%S"))
            } else if test_id == "MDPING" {
                let host = params.get("ping_host").and_then(|v| v.as_str()).unwrap_or("8.8.8.8");
                let count = params.get("ping_count").and_then(|v| v.as_i64()).unwrap_or(3);
                format!("AT+MDPING=\"{}\",{}", host, count)
            } else {
                let static_cmd = get_at_command(test_id);
                if static_cmd.is_empty() {
                    continue;
                }
                static_cmd.to_string()
            };

            let attempts = 1 + retries;
            let mut last_result = None;

            for attempt in 0..attempts {
                if attempt > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                let start = std::time::Instant::now();
                match mgr.send_command(&cmd, timeout) {
                    Ok(resp) => {
                        let duration = start.elapsed().as_millis() as u64;
                        let (status, error) = judge_test(test_id, &resp, &params);
                        let parsed_data = resp
                            .lines
                            .first()
                            .map(|l| parse_kv_response(l))
                            .unwrap_or_default();

                        let result = TestResult {
                            id: test_id.clone(),
                            command: cmd.clone(),
                            status: status.clone(),
                            raw_response: resp.lines.join("\n"),
                            parsed_data,
                            error,
                            duration_ms: duration,
                        };

                        if status == "pass" {
                            last_result = Some(result);
                            break;
                        }
                        last_result = Some(result);
                    }
                    Err(e) => {
                        last_result = Some(TestResult {
                            id: test_id.clone(),
                            command: cmd.clone(),
                            status: "fail".to_string(),
                            raw_response: String::new(),
                            parsed_data: HashMap::new(),
                            error: e.to_string(),
                            duration_ms: start.elapsed().as_millis() as u64,
                        });
                    }
                }
            }

            if let Some(r) = last_result {
                let _ = app_handle.emit("test:result", &r);
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub fn manual_judge(_test_id: String, _pass: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn config_read(serial: State<'_, SharedSerialManager>) -> Result<Vec<String>, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+CFGREAD?", get_timeout_ms("CFGREAD"))
            .map_err(|e| e.to_string())?;

        if !resp.success {
            return Err("读取配置失败".to_string());
        }

        let lines: Vec<String> = resp
            .lines
            .iter()
            .filter_map(|l| {
                l.strip_prefix("+CFGREAD:")
                    .or_else(|| l.strip_prefix("+CFGREAD: "))
                    .map(|s| s.trim().to_string())
            })
            .collect();

        Ok(lines)
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn config_info(
    serial: State<'_, SharedSerialManager>,
) -> Result<ConfigInfo, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+CFGINFO?", get_timeout_ms("CFGINFO"))
            .map_err(|e| e.to_string())?;

        if !resp.success {
            return Err("查询配置信息失败".to_string());
        }

        let data = resp
            .lines
            .first()
            .map(|l| parse_kv_response(l))
            .unwrap_or_default();

        Ok(ConfigInfo {
            exist: data.get("EXIST").map(|v| v == "1").unwrap_or(false),
            size: data
                .get("SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            version: data
                .get("VERSION")
                .cloned()
                .unwrap_or_else(|| "0".to_string()),
        })
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn config_upload(
    serial: State<'_, SharedSerialManager>,
    lines: Vec<String>,
) -> Result<ConfigUploadResult, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();

        let resp = mgr
            .send_command("AT+CFGSTART", get_timeout_ms("CFGSTART"))
            .map_err(|e| e.to_string())?;
        if !resp.success {
            return Err("CFGSTART 失败".to_string());
        }

        let mut lines_sent = 0;
        for line in &lines {
            let cmd = format!("AT+CFGSET=\"{}\"", line);
            let resp = mgr
                .send_command(&cmd, get_timeout_ms("CFGSET"))
                .map_err(|e| e.to_string())?;
            if !resp.success {
                return Err(format!("CFGSET 第{}行失败", lines_sent + 1));
            }
            lines_sent += 1;
        }

        let resp = mgr
            .send_command("AT+CFGSAVE", get_timeout_ms("CFGSAVE"))
            .map_err(|e| e.to_string())?;
        if !resp.success {
            return Err(format!(
                "CFGSAVE 失败: {}",
                resp.error_code.unwrap_or_default()
            ));
        }

        let readback_resp = mgr
            .send_command("AT+CFGREAD?", get_timeout_ms("CFGREAD"))
            .map_err(|e| e.to_string())?;

        let readback: Vec<String> = readback_resp
            .lines
            .iter()
            .filter_map(|l| {
                l.strip_prefix("+CFGREAD:")
                    .or_else(|| l.strip_prefix("+CFGREAD: "))
                    .map(|s| s.trim().to_string())
            })
            .collect();

        Ok(ConfigUploadResult {
            lines_sent,
            readback,
        })
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn config_restore_default(
    serial: State<'_, SharedSerialManager>,
) -> Result<(), String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+CFGDEF", get_timeout_ms("CFGDEF"))
            .map_err(|e| e.to_string())?;
        if !resp.success {
            return Err("恢复默认配置失败".to_string());
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn config_clear(serial: State<'_, SharedSerialManager>) -> Result<(), String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();

        let resp = mgr
            .send_command("AT+CFGSTART", get_timeout_ms("CFGSTART"))
            .map_err(|e| e.to_string())?;
        if !resp.success {
            return Err("CFGSTART 失败".to_string());
        }

        let resp = mgr
            .send_command("AT+CFGSAVE", get_timeout_ms("CFGSAVE"))
            .map_err(|e| e.to_string())?;
        if !resp.success {
            return Err("CFGSAVE 失败".to_string());
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

// --- Report commands ---

#[tauri::command]
pub fn cmd_save_report(report_data: TestReport) -> Result<String, String> {
    report::save_report(&report_data)
}

#[tauri::command]
pub fn cmd_list_reports(filter: ReportFilter) -> Result<Vec<ReportSummary>, String> {
    report::list_reports(&filter)
}

#[tauri::command]
pub fn cmd_get_report(report_id: String) -> Result<TestReport, String> {
    report::get_report(&report_id)
}

#[tauri::command]
pub fn cmd_delete_report(report_id: String) -> Result<(), String> {
    report::delete_report(&report_id)
}

#[tauri::command]
pub fn cmd_export_csv(filter: ReportFilter, path: Option<String>) -> Result<String, String> {
    report::export_csv(&filter, path.as_deref())
}

#[tauri::command]
pub fn cmd_get_report_path(report_id: String) -> Result<String, String> {
    report::get_report_path(&report_id)
}

// --- Settings commands ---

#[tauri::command]
pub fn cmd_load_settings() -> AppSettings {
    settings::load_settings()
}

#[tauri::command]
pub fn cmd_save_settings(settings_data: AppSettings) -> Result<(), String> {
    settings::save_settings(&settings_data)
}

#[tauri::command]
pub fn cmd_get_data_dir() -> String {
    report::get_data_dir()
}

#[tauri::command]
pub fn cmd_get_default_test_items() -> Vec<TestItemConfig> {
    default_test_items()
}

// --- Template commands ---

#[tauri::command]
pub fn cmd_list_templates() -> Result<Vec<ConfigTemplate>, String> {
    settings::list_templates()
}

#[tauri::command]
pub fn cmd_save_template(template: ConfigTemplate) -> Result<(), String> {
    settings::save_template(&template)
}

#[tauri::command]
pub fn cmd_delete_template(name: String) -> Result<(), String> {
    settings::delete_template(&name)
}

// --- Device info command ---

#[tauri::command]
pub async fn query_device_info(
    serial: State<'_, SharedSerialManager>,
) -> Result<HashMap<String, String>, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+MDINFO?", get_timeout_ms("MDINFO"))
            .map_err(|e| e.to_string())?;

        if !resp.success {
            return Err("查询设备信息失败".to_string());
        }

        let mut data = HashMap::new();
        for line in &resp.lines {
            let parsed = parse_kv_response(line);
            data.extend(parsed);
        }

        Ok(data)
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub async fn query_version(
    serial: State<'_, SharedSerialManager>,
) -> Result<HashMap<String, String>, String> {
    let serial = serial.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+VER?", get_timeout_ms("VER"))
            .map_err(|e| e.to_string())?;

        if !resp.success {
            return Err("查询版本失败".to_string());
        }

        let mut data = HashMap::new();
        for line in &resp.lines {
            let parsed = parse_kv_response(line);
            data.extend(parsed);
        }

        Ok(data)
    })
    .await
    .map_err(|e| format!("内部错误: {}", e))?
}

#[tauri::command]
pub fn cmd_load_product_config(product: String) -> product_config::ProductConfig {
    product_config::load_product_config(&product)
}
