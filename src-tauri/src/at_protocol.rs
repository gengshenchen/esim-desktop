use crate::error::AppError;
use crate::types::{ATResponse, DeviceCapability};
use std::collections::HashMap;

pub fn parse_cap_response(resp: &ATResponse) -> Result<DeviceCapability, AppError> {
    let mut product = String::new();
    let mut config = false;
    let mut production = false;
    let mut mcu = false;

    for line in &resp.lines {
        let data = line
            .strip_prefix("+CAP:")
            .or_else(|| line.strip_prefix("+CAP: "))
            .unwrap_or(line)
            .trim();

        for part in data.split(',') {
            let kv: Vec<&str> = part.split('=').collect();
            if kv.len() == 2 {
                match kv[0].trim() {
                    "PRODUCT" => product = kv[1].trim().to_string(),
                    "CONFIG" => config = kv[1].trim() == "1",
                    "PRODUCTION" => production = kv[1].trim() == "1",
                    "MCU" => mcu = kv[1].trim() == "1",
                    _ => {}
                }
            }
        }
    }

    if product.is_empty() {
        return Err(AppError::Protocol("无法解析设备能力".to_string()));
    }

    Ok(DeviceCapability {
        product,
        config,
        production,
        mcu,
    })
}

pub fn parse_kv_response(line: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    let data = if let Some(pos) = line.find(':') {
        line[pos + 1..].trim()
    } else {
        line.trim()
    };

    for part in data.split(',') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 {
            map.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
        } else if !part.trim().is_empty() {
            map.insert("value".to_string(), part.trim().to_string());
        }
    }

    map
}

pub fn get_timeout_ms(cmd_id: &str) -> u64 {
    match cmd_id {
        "CAP" | "VER" | "DEV" => 3000,
        "PROD" => 8000,
        "CFGINFO" | "CFGSTART" | "CFGSET" | "CFGDEF" => 3000,
        "CFGSAVE" | "CFGREAD" => 5000,
        "MDINFO" => 5000,
        "MDSIM" | "MDREG" | "MDSIG" | "MDDATA" => 5000,
        "MDDNS" => 10000,
        "MDPING" => 20000,
        "MDALL" => 30000,
        "MCUBVER" | "MCUMAC" | "MCUCHG" | "MCUVBAT" | "MCUTIME" => 5000,
        "MCULED" | "MCUFBMIC" | "MCUPMIC" | "MCUKEY" => 5000,
        "MCUGAUGE" | "MCURST" | "MCURAW" => 8000,
        _ => 5000,
    }
}

pub fn get_at_command(test_id: &str) -> &'static str {
    match test_id {
        "MDSIM" => "AT+MDSIM?",
        "MDREG" => "AT+MDREG?",
        "MDSIG" => "AT+MDSIG?",
        "MDDATA" => "AT+MDDATA?",
        "MDALL" => "AT+MDALL?",
        "MDINFO" => "AT+MDINFO?",
        "MCUBVER" => "AT+MCUBVER?",
        "MCUMAC" => "AT+MCUMAC?",
        "MCUCHG" => "AT+MCUCHG?",
        "MCUVBAT" => "AT+MCUVBAT?",
        "MCULED" => "AT+MCULED=1",
        "MCUFBMIC" => "AT+MCUFBMIC=1",
        "MCUPMIC" => "AT+MCUPMIC=1",
        "MCUKEY" => "AT+MCUKEY",
        "MCUGAUGE" => "AT+MCUGAUGE",
        "MCURST" => "AT+MCURST",
        _ => "",
    }
}

pub fn judge_test(
    test_id: &str,
    resp: &ATResponse,
    params: &HashMap<String, serde_json::Value>,
) -> (String, String) {
    if !resp.success {
        let err = resp.error_code.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        return ("fail".to_string(), err);
    }

    let combined = resp.lines.join(" ");

    match test_id {
        "MDSIM" => {
            if combined.contains("READY") {
                ("pass".to_string(), String::new())
            } else {
                ("fail".to_string(), "SIM 未就绪".to_string())
            }
        }
        "MDREG" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let creg = data.get("CREG").map(|v| v.as_str()).unwrap_or("0");
            let cgreg = data.get("CGREG").map(|v| v.as_str()).unwrap_or("0");
            let creg_ok = matches!(creg, "1" | "5" | "6");
            let cgreg_ok = matches!(cgreg, "1" | "5" | "6");
            if creg_ok && cgreg_ok {
                ("pass".to_string(), String::new())
            } else {
                ("fail".to_string(), format!("CREG={},CGREG={}", creg, cgreg))
            }
        }
        "MDSIG" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let csq: i32 = data.get("CSQ").and_then(|v| v.parse().ok()).unwrap_or(-999);
            let rsrp: i32 = data.get("RSRP").and_then(|v| v.parse().ok()).unwrap_or(-999);
            let csq_min = params.get("csq_min").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
            let rssi_min = params.get("rssi_min").and_then(|v| v.as_i64()).unwrap_or(-90) as i32;
            let rsrp_min = params.get("rsrp_min").and_then(|v| v.as_i64()).unwrap_or(-110) as i32;
            let csq_ok = if csq >= 0 { csq >= csq_min } else { csq >= rssi_min };
            let rsrp_ok = rsrp >= rsrp_min;
            if csq_ok && rsrp_ok {
                ("pass".to_string(), String::new())
            } else {
                let mut reasons = Vec::new();
                if !csq_ok {
                    if csq >= 0 {
                        reasons.push(format!("CSQ={}(需≥{})", csq, csq_min));
                    } else {
                        reasons.push(format!("CSQ={}dBm(需≥{})", csq, rssi_min));
                    }
                }
                if !rsrp_ok {
                    reasons.push(format!("RSRP={}(需≥{})", rsrp, rsrp_min));
                }
                ("fail".to_string(), reasons.join(","))
            }
        }
        "MDDATA" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let state = data.get("STATE").map(|v| v.as_str()).unwrap_or("");
            let ip = data.get("IP").map(|v| v.as_str()).unwrap_or("");
            if state == "UP" && !ip.is_empty() {
                ("pass".to_string(), String::new())
            } else {
                ("fail".to_string(), format!("STATE={},IP={}", state, ip))
            }
        }
        "MDALL" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let ping_enabled = params.get("ping_enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let all_ok = data.iter().all(|(k, v)| {
                if !ping_enabled && (k == "PING" || k == "DNS") {
                    return true;
                }
                v == "OK"
            });
            if all_ok {
                ("pass".to_string(), String::new())
            } else {
                let failures: Vec<String> = data
                    .iter()
                    .filter(|(k, v)| {
                        if !ping_enabled && (k.as_str() == "PING" || k.as_str() == "DNS") {
                            return false;
                        }
                        v.as_str() != "OK"
                    })
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                ("fail".to_string(), failures.join(","))
            }
        }
        "MCUVBAT" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let mv: i32 = data.get("MV").and_then(|v| v.parse().ok()).unwrap_or(0);
            let mv_min = params.get("mv_min").and_then(|v| v.as_i64()).unwrap_or(3000) as i32;
            let mv_max = params.get("mv_max").and_then(|v| v.as_i64()).unwrap_or(4500) as i32;
            if mv >= mv_min && mv <= mv_max {
                ("pass".to_string(), String::new())
            } else {
                ("fail".to_string(), format!("MV={} (范围{}-{})", mv, mv_min, mv_max))
            }
        }
        "MCUMAC" => {
            let data = resp.lines.first().map(|l| parse_kv_response(l)).unwrap_or_default();
            let mac = data.get("MAC").map(|v| v.as_str()).unwrap_or("");
            if mac.len() == 17 && mac.contains(':') {
                ("pass".to_string(), String::new())
            } else {
                ("fail".to_string(), format!("MAC格式无效: {}", mac))
            }
        }
        "MCULED" | "MCUFBMIC" | "MCUPMIC" => {
            ("manual_pending".to_string(), String::new())
        }
        "MCUKEY" => {
            ("manual_pending".to_string(), String::new())
        }
        _ => {
            ("pass".to_string(), String::new())
        }
    }
}
