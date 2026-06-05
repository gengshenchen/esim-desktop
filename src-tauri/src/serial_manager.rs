use crate::error::AppError;
use crate::types::{ATResponse, SerialDataEvent};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

fn emit_event(handle: &Option<AppHandle>, line: &str, direction: &str) {
    if let Some(ref h) = handle {
        let _ = h.emit(
            "serial:data",
            SerialDataEvent {
                line: line.to_string(),
                direction: direction.to_string(),
            },
        );
    }
}

pub struct SerialManager {
    port: Option<Box<dyn SerialPort>>,
    app_handle: Option<AppHandle>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    pub fn scan_ports(&self) -> Vec<String> {
        match serialport::available_ports() {
            Ok(ports) => ports
                .into_iter()
                .map(|p| p.port_name)
                .filter(|name| {
                    name.contains("ttyACM") || name.contains("ttyUSB") || name.contains("COM")
                })
                .collect(),
            Err(_) => vec![],
        }
    }

    pub fn connect(&mut self, port_name: &str) -> Result<(), AppError> {
        let port = serialport::new(port_name, 115200)
            .timeout(Duration::from_millis(100))
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .open()?;

        self.port = Some(port);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.port = None;
    }

    pub fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    pub fn send_command(
        &mut self,
        cmd: &str,
        timeout_ms: u64,
    ) -> Result<ATResponse, AppError> {
        let handle = self.app_handle.clone();

        let port = self
            .port
            .as_mut()
            .ok_or(AppError::NotConnected)?;

        let cmd_with_crlf = if cmd.ends_with("\r\n") {
            cmd.to_string()
        } else {
            format!("{}\r\n", cmd)
        };

        let mut drain_buf = [0u8; 1024];
        loop {
            match port.read(&mut drain_buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        emit_event(&handle, cmd, "tx");

        port.write_all(cmd_with_crlf.as_bytes())
            .map_err(|e| AppError::Serial(e.to_string()))?;
        port.flush()
            .map_err(|e| AppError::Serial(e.to_string()))?;

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let mut buffer = Vec::new();
        let mut read_buf = [0u8; 1024];
        let mut lines: Vec<String> = Vec::new();
        let mut success = false;
        let mut finished = false;

        while start.elapsed() < timeout && !finished {
            match port.read(&mut read_buf) {
                Ok(n) if n > 0 => {
                    buffer.extend_from_slice(&read_buf[..n]);

                    while let Some(pos) = buffer
                        .windows(2)
                        .position(|w| w == b"\r\n")
                    {
                        let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                        buffer.drain(..pos + 2);

                        if line.is_empty() {
                            continue;
                        }

                        emit_event(&handle, &line, "rx");

                        if line == "OK" {
                            success = true;
                            finished = true;
                            break;
                        } else if line == "ERROR" {
                            success = false;
                            finished = true;
                            break;
                        } else {
                            lines.push(line);
                        }
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(AppError::Serial(e.to_string())),
            }
        }

        if !finished {
            return Err(AppError::Timeout(format!(
                "命令 {} 超时 ({}ms)",
                cmd, timeout_ms
            )));
        }

        let error_code = if !success {
            lines
                .last()
                .and_then(|l| {
                    if let Some(pos) = l.find("ERR,") {
                        Some(l[pos + 4..].to_string())
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        Ok(ATResponse {
            lines,
            success,
            error_code,
        })
    }
}

pub type SharedSerialManager = Arc<Mutex<SerialManager>>;

pub fn create_serial_manager() -> SharedSerialManager {
    Arc::new(Mutex::new(SerialManager::new()))
}
