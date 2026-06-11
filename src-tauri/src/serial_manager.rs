use crate::error::AppError;
use crate::types::{ATResponse, KeyEvent, SerialDataEvent};
use serialport::SerialPort;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

fn emit_serial_data(handle: &AppHandle, line: &str, direction: &str) {
    let _ = handle.emit(
        "serial:data",
        SerialDataEvent {
            line: line.to_string(),
            direction: direction.to_string(),
        },
    );
}

fn parse_key_urc(line: &str) -> Option<KeyEvent> {
    // +MCUKEY:KEY=MULTI_FUN,STATE=PRESS
    let data = line.strip_prefix("+MCUKEY:")?;
    let mut key = String::new();
    let mut state = String::new();
    for part in data.split(',') {
        if let Some(v) = part.strip_prefix("KEY=") {
            key = v.to_string();
        } else if let Some(v) = part.strip_prefix("STATE=") {
            state = v.to_string();
        }
    }
    if key.is_empty() || state.is_empty() {
        return None;
    }
    Some(KeyEvent { key, state })
}

fn is_key_urc(line: &str) -> bool {
    line.starts_with("+MCUKEY:KEY=")
}

fn reader_thread(
    mut read_port: Box<dyn SerialPort>,
    response_tx: Sender<String>,
    app_handle: AppHandle,
    stop_flag: Arc<AtomicBool>,
) {
    let mut buffer = Vec::with_capacity(4096);
    let mut read_buf = [0u8; 1024];

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        match read_port.read(&mut read_buf) {
            Ok(0) => {}
            Ok(n) => {
                buffer.extend_from_slice(&read_buf[..n]);

                while let Some(pos) = buffer.windows(2).position(|w| w == b"\r\n") {
                    let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                    buffer.drain(..pos + 2);

                    if line.is_empty() {
                        continue;
                    }

                    emit_serial_data(&app_handle, &line, "rx");

                    if is_key_urc(&line) {
                        if let Some(kv) = parse_key_urc(&line) {
                            let _ = app_handle.emit("key:event", &kv);
                        }
                    } else {
                        let _ = response_tx.send(line);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                continue;
            }
            Err(_) => {
                if !stop_flag.load(Ordering::Relaxed) {
                    let _ = app_handle.emit("serial:disconnected", ());
                }
                break;
            }
        }
    }
}

pub struct SerialManager {
    write_port: Option<Box<dyn SerialPort>>,
    response_rx: Option<Receiver<String>>,
    app_handle: Option<AppHandle>,
    connected_port: Option<String>,
    reader_stop: Arc<AtomicBool>,
    reader_thread: Option<std::thread::JoinHandle<()>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            write_port: None,
            response_rx: None,
            app_handle: None,
            connected_port: None,
            reader_stop: Arc::new(AtomicBool::new(false)),
            reader_thread: None,
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

        let read_port = port
            .try_clone()
            .map_err(|e| AppError::Serial(format!("try_clone failed: {}", e)))?;

        self.write_port = Some(port);
        self.connected_port = Some(port_name.to_string());

        let (response_tx, response_rx) = mpsc::channel();
        self.response_rx = Some(response_rx);

        let stop_flag = Arc::new(AtomicBool::new(false));
        self.reader_stop = stop_flag.clone();

        let app_handle = self
            .app_handle
            .clone()
            .ok_or(AppError::Serial("app_handle not set".to_string()))?;

        let handle = std::thread::spawn(move || {
            reader_thread(read_port, response_tx, app_handle, stop_flag);
        });
        self.reader_thread = Some(handle);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.reader_stop.store(true, Ordering::Relaxed);

        // Drop write port first to unblock reader if it's stuck on read
        self.write_port = None;
        self.response_rx = None;

        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }

        self.connected_port = None;
        self.reader_stop = Arc::new(AtomicBool::new(false));
    }

    pub fn is_connected(&self) -> bool {
        self.write_port.is_some()
    }

    pub fn connected_port_name(&self) -> Option<String> {
        self.connected_port.clone()
    }

    pub fn force_disconnect(&mut self) {
        self.reader_stop.store(true, Ordering::Relaxed);
        self.write_port = None;
        self.response_rx = None;
        self.connected_port = None;
        // Don't join reader thread here — called from disconnect event handler
        self.reader_thread = None;
        self.reader_stop = Arc::new(AtomicBool::new(false));
    }

    pub fn send_command(
        &mut self,
        cmd: &str,
        timeout_ms: u64,
    ) -> Result<ATResponse, AppError> {
        let write_port = self
            .write_port
            .as_mut()
            .ok_or(AppError::NotConnected)?;

        let response_rx = self
            .response_rx
            .as_ref()
            .ok_or(AppError::NotConnected)?;

        // Drain any leftover lines from previous command
        while response_rx.try_recv().is_ok() {}

        let cmd_with_crlf = if cmd.ends_with("\r\n") {
            cmd.to_string()
        } else {
            format!("{}\r\n", cmd)
        };

        if let Some(ref h) = self.app_handle {
            emit_serial_data(h, cmd, "tx");
        }

        write_port
            .write_all(cmd_with_crlf.as_bytes())
            .map_err(|e| AppError::Serial(e.to_string()))?;
        write_port
            .flush()
            .map_err(|e| AppError::Serial(e.to_string()))?;

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let mut lines: Vec<String> = Vec::new();
        let mut success = false;
        let mut finished = false;

        while start.elapsed() < timeout && !finished {
            let remaining = timeout.saturating_sub(start.elapsed());
            let recv_timeout = remaining.min(Duration::from_millis(100));

            match response_rx.recv_timeout(recv_timeout) {
                Ok(line) => {
                    if line == "OK" {
                        success = true;
                        finished = true;
                    } else if line == "ERROR" {
                        success = false;
                        finished = true;
                    } else {
                        lines.push(line);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(AppError::Serial("串口读取线程已断开".to_string()));
                }
            }
        }

        if !finished {
            return Err(AppError::Timeout(format!(
                "命令 {} 超时 ({}ms)",
                cmd, timeout_ms
            )));
        }

        let error_code = if !success {
            lines.last().and_then(|l| {
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
