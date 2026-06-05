use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Serial(String),
    Protocol(String),
    Timeout(String),
    NotConnected,
    NotInProductionMode,
    Io(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Serial(msg) => write!(f, "串口错误: {}", msg),
            AppError::Protocol(msg) => write!(f, "协议错误: {}", msg),
            AppError::Timeout(msg) => write!(f, "超时: {}", msg),
            AppError::NotConnected => write!(f, "未连接设备"),
            AppError::NotInProductionMode => write!(f, "未进入产测模式"),
            AppError::Io(msg) => write!(f, "IO错误: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<serialport::Error> for AppError {
    fn from(e: serialport::Error) -> Self {
        AppError::Serial(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl Into<String> for AppError {
    fn into(self) -> String {
        self.to_string()
    }
}
