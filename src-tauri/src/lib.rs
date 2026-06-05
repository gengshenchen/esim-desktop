mod at_protocol;
mod commands;
mod error;
mod serial_manager;
mod types;

use serial_manager::{create_serial_manager, SharedSerialManager};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(create_serial_manager())
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
