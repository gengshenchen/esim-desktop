use crate::types::{ReportFilter, ReportSummary, TestReport};
use std::fs;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".esim-production-tool")
}

fn reports_dir() -> PathBuf {
    data_dir().join("reports")
}

pub fn ensure_dirs() {
    let _ = fs::create_dir_all(reports_dir());
    let _ = fs::create_dir_all(data_dir().join("templates"));
}

pub fn save_report(report: &TestReport) -> Result<String, String> {
    ensure_dirs();

    let date = if report.timestamp.len() >= 10 {
        &report.timestamp[..10]
    } else {
        "unknown"
    };
    let day_dir = reports_dir().join(date);
    fs::create_dir_all(&day_dir).map_err(|e| format!("创建报告目录失败: {}", e))?;

    let filename = format!("{}.json", report.id);
    let path = day_dir.join(&filename);

    let json = serde_json::to_string_pretty(report)
        .map_err(|e| format!("序列化报告失败: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("写入报告失败: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

pub fn list_reports(filter: &ReportFilter) -> Result<Vec<ReportSummary>, String> {
    ensure_dirs();
    let base = reports_dir();
    let mut summaries = Vec::new();

    let mut day_dirs: Vec<_> = fs::read_dir(&base)
        .map_err(|e| format!("读取报告目录失败: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    day_dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for day_dir in day_dirs {
        let dir_name = day_dir.file_name().to_string_lossy().to_string();

        if let Some(ref from) = filter.date_from {
            if dir_name < *from {
                continue;
            }
        }
        if let Some(ref to) = filter.date_to {
            if dir_name > *to {
                continue;
            }
        }

        let mut files: Vec<_> = fs::read_dir(day_dir.path())
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
            .collect();
        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        for file in files {
            let content = match fs::read_to_string(file.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let report: TestReport = match serde_json::from_str(&content) {
                Ok(r) => r,
                Err(_) => continue,
            };

            if let Some(ref result) = filter.result {
                if !result.is_empty() && report.overall != *result {
                    continue;
                }
            }

            if let Some(ref search) = filter.search {
                if !search.is_empty() {
                    let s = search.to_lowercase();
                    let matches = report.device.imei.to_lowercase().contains(&s)
                        || report.device.bt_mac.to_lowercase().contains(&s)
                        || report.device.product.to_lowercase().contains(&s)
                        || report.operator.to_lowercase().contains(&s);
                    if !matches {
                        continue;
                    }
                }
            }

            let pass_count = report.items.iter().filter(|i| i.status == "pass").count();
            let fail_count = report.items.iter().filter(|i| i.status == "fail").count();

            summaries.push(ReportSummary {
                id: report.id,
                timestamp: report.timestamp,
                imei: report.device.imei,
                product: report.device.product,
                overall: report.overall,
                operator: report.operator,
                pass_count,
                fail_count,
                total_count: report.items.len(),
            });
        }
    }

    Ok(summaries)
}

pub fn get_report(report_id: &str) -> Result<TestReport, String> {
    let base = reports_dir();

    let day_dirs = fs::read_dir(&base).map_err(|e| e.to_string())?;
    for entry in day_dirs.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path().join(format!("{}.json", report_id));
        if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let report: TestReport =
                serde_json::from_str(&content).map_err(|e| e.to_string())?;
            return Ok(report);
        }
    }

    Err(format!("报告 {} 不存在", report_id))
}

pub fn delete_report(report_id: &str) -> Result<(), String> {
    let base = reports_dir();

    let day_dirs = fs::read_dir(&base).map_err(|e| e.to_string())?;
    for entry in day_dirs.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path().join(format!("{}.json", report_id));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err(format!("报告 {} 不存在", report_id))
}

pub fn export_csv(filter: &ReportFilter, path: Option<&str>) -> Result<String, String> {
    let reports = list_reports(filter)?;
    let mut csv = String::from("序号,时间,IMEI,产品,操作员,结果,通过,失败,总计\n");

    for (i, r) in reports.iter().enumerate() {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            r.timestamp,
            r.imei,
            r.product,
            r.operator,
            r.overall,
            r.pass_count,
            r.fail_count,
            r.total_count,
        ));
    }

    let export_path = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => data_dir().join("reports_export.csv"),
    };

    if let Some(parent) = export_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&export_path, &csv).map_err(|e| format!("导出失败: {}", e))?;

    Ok(export_path.to_string_lossy().to_string())
}

pub fn get_data_dir() -> String {
    data_dir().to_string_lossy().to_string()
}

pub fn get_report_path(report_id: &str) -> Result<String, String> {
    let base = reports_dir();
    let day_dirs = fs::read_dir(&base).map_err(|e| e.to_string())?;
    for entry in day_dirs.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path().join(format!("{}.json", report_id));
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }
    Err(format!("报告 {} 不存在", report_id))
}
