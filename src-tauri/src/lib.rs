mod at_protocol;
mod commands;
mod error;
mod report;
mod serial_manager;
mod settings;
mod types;

use serial_manager::{create_serial_manager, start_port_monitor, SharedSerialManager};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    report::ensure_dirs();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(create_serial_manager())
        .setup(|app| {
            let serial = app.state::<SharedSerialManager>().inner().clone();
            start_port_monitor(serial, app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_ports,
            commands::connect,
            commands::disconnect,
            commands::send_at_command,
            commands::enter_production_mode,
            commands::exit_production_mode,
            commands::run_single_test,
            commands::run_auto_test,
            commands::manual_judge,
            commands::config_read,
            commands::config_info,
            commands::config_upload,
            commands::config_restore_default,
            commands::config_clear,
            commands::cmd_save_report,
            commands::cmd_list_reports,
            commands::cmd_get_report,
            commands::cmd_delete_report,
            commands::cmd_export_csv,
            commands::cmd_load_settings,
            commands::cmd_save_settings,
            commands::cmd_get_data_dir,
            commands::cmd_list_templates,
            commands::cmd_save_template,
            commands::cmd_delete_template,
            commands::cmd_get_default_test_items,
            commands::query_device_info,
            commands::query_version,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let serial = window.state::<SharedSerialManager>();
                let mut mgr = serial.lock().unwrap();
                if mgr.is_connected() {
                    let _ = mgr.send_command(
                        "AT+PROD=0",
                        at_protocol::get_timeout_ms("PROD"),
                    );
                    mgr.disconnect();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
