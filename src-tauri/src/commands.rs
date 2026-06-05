use crate::at_protocol::{
    get_at_command, get_timeout_ms, judge_test, parse_cap_response, parse_kv_response,
};
use crate::serial_manager::SharedSerialManager;
use crate::types::*;
use std::collections::HashMap;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn scan_ports(serial: State<SharedSerialManager>) -> Vec<String> {
    let mgr = serial.lock().unwrap();
    mgr.scan_ports()
}

#[tauri::command]
pub fn connect(
    serial: State<SharedSerialManager>,
    port: String,
    app_handle: AppHandle,
) -> Result<DeviceCapability, String> {
    let mut mgr = serial.lock().unwrap();
    mgr.set_app_handle(app_handle);
    mgr.connect(&port).map_err(|e| format!("打开串口失败: {}", e))?;

    // 短暂等待设备就绪
    std::thread::sleep(std::time::Duration::from_millis(200));

    let resp = mgr
        .send_command("AT+CAP?", get_timeout_ms("CAP"))
        .map_err(|e| {
            mgr.disconnect();
            format!("串口已打开但设备未响应 AT+CAP? ({})", e)
        })?;

    parse_cap_response(&resp).map_err(|e| {
        mgr.disconnect();
        format!("设备响应格式异常: {}", e)
    })
}

#[tauri::command]
pub fn disconnect(serial: State<SharedSerialManager>) -> Result<(), String> {
    let mut mgr = serial.lock().unwrap();
    if mgr.is_connected() {
        let _ = mgr.send_command("AT+PROD=0", get_timeout_ms("PROD"));
    }
    mgr.disconnect();
    Ok(())
}

#[tauri::command]
pub fn send_at_command(
    serial: State<SharedSerialManager>,
    cmd: String,
    timeout_ms: u64,
) -> Result<ATResponse, String> {
    let mut mgr = serial.lock().unwrap();
    mgr.send_command(&cmd, timeout_ms)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enter_production_mode(serial: State<SharedSerialManager>) -> Result<ATResponse, String> {
    let max_retries = 3;
    let mut last_err = String::new();

    for attempt in 0..max_retries {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        let mut mgr = serial.lock().unwrap();
        match mgr.send_command("AT+PROD=1", get_timeout_ms("PROD")) {
            Ok(resp) if resp.success => return Ok(resp),
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

    Err(format!("进入产测模式失败 (重试{}次): {}", max_retries, last_err))
}

#[tauri::command]
pub fn exit_production_mode(serial: State<SharedSerialManager>) -> Result<ATResponse, String> {
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
}

#[tauri::command]
pub fn run_single_test(
    serial: State<SharedSerialManager>,
    test_id: String,
) -> Result<TestResult, String> {
    let cmd = if test_id == "MCUTIME" {
        let now = chrono::Local::now();
        format!("AT+MCUTIME=\"{}\"", now.format("%Y-%m-%d %H:%M:%S"))
    } else {
        let static_cmd = get_at_command(&test_id);
        if static_cmd.is_empty() {
            return Err(format!("未知测试项: {}", test_id));
        }
        static_cmd.to_string()
    };

    let timeout = get_timeout_ms(&test_id);
    let mut mgr = serial.lock().unwrap();

    let start = std::time::Instant::now();
    let resp = mgr.send_command(&cmd, timeout).map_err(|e| e.to_string())?;
    let duration = start.elapsed().as_millis() as u64;

    let (status, error) = judge_test(&test_id, &resp);
    let parsed_data = resp
        .lines
        .first()
        .map(|l| parse_kv_response(l))
        .unwrap_or_default();

    Ok(TestResult {
        id: test_id,
        status,
        raw_response: resp.lines.join("\n"),
        parsed_data,
        error,
        duration_ms: duration,
    })
}

#[tauri::command]
pub fn run_auto_test(serial: State<SharedSerialManager>) -> Result<Vec<TestResult>, String> {
    // 1. 进入产测模式 AT+PROD=1
    {
        let mut mgr = serial.lock().unwrap();
        let resp = mgr
            .send_command("AT+PROD=1", get_timeout_ms("PROD"))
            .map_err(|e| format!("进入产测模式失败: {}", e))?;
        if !resp.success {
            return Err(format!(
                "进入产测模式失败: {}",
                resp.error_code.unwrap_or_else(|| "未知错误".to_string())
            ));
        }
    }

    // 2. 执行测试序列
    let test_ids = vec![
        "MDSIM", "MDREG", "MDSIG", "MDDATA", "MDALL", "MCUBVER", "MCUMAC", "MCUCHG", "MCUVBAT",
    ];

    let mut results = Vec::new();

    for test_id in test_ids {
        let cmd = get_at_command(test_id);
        if cmd.is_empty() {
            continue;
        }

        let timeout = get_timeout_ms(test_id);
        let mut mgr = serial.lock().unwrap();

        let start = std::time::Instant::now();
        match mgr.send_command(cmd, timeout) {
            Ok(resp) => {
                let duration = start.elapsed().as_millis() as u64;
                let (status, error) = judge_test(test_id, &resp);
                let parsed_data = resp
                    .lines
                    .first()
                    .map(|l| parse_kv_response(l))
                    .unwrap_or_default();

                results.push(TestResult {
                    id: test_id.to_string(),
                    status,
                    raw_response: resp.lines.join("\n"),
                    parsed_data,
                    error,
                    duration_ms: duration,
                });
            }
            Err(e) => {
                results.push(TestResult {
                    id: test_id.to_string(),
                    status: "fail".to_string(),
                    raw_response: String::new(),
                    parsed_data: HashMap::new(),
                    error: e.to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        drop(mgr);
    }

    // 3. 退出产测模式 AT+PROD=0 (无论测试是否全部成功都要退出)
    {
        let mut mgr = serial.lock().unwrap();
        let _ = mgr.send_command("AT+PROD=0", get_timeout_ms("PROD"));
    }

    Ok(results)
}

#[tauri::command]
pub fn manual_judge(_test_id: String, _pass: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn config_read(serial: State<SharedSerialManager>) -> Result<Vec<String>, String> {
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
}

#[tauri::command]
pub fn config_info(serial: State<SharedSerialManager>) -> Result<ConfigInfo, String> {
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
}

#[tauri::command]
pub fn config_upload(
    serial: State<SharedSerialManager>,
    lines: Vec<String>,
) -> Result<ConfigUploadResult, String> {
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
}

#[tauri::command]
pub fn config_restore_default(serial: State<SharedSerialManager>) -> Result<(), String> {
    let mut mgr = serial.lock().unwrap();
    let resp = mgr
        .send_command("AT+CFGDEF", get_timeout_ms("CFGDEF"))
        .map_err(|e| e.to_string())?;
    if !resp.success {
        return Err("恢复默认配置失败".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn config_clear(serial: State<SharedSerialManager>) -> Result<(), String> {
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
}
