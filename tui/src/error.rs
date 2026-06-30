use std::fmt;

/// App 错误类型
#[derive(Debug)]
pub enum AppError {
    /// API 请求失败
    Api(reqwest::Error),
    /// 配置加载失败
    Config(ConfigError),
    /// IO 错误
    Io(std::io::Error),
    /// 输入验证失败
    Validation(String),
    /// 内核未运行
    KernelNotRunning,
    /// 内核已运行
    KernelAlreadyRunning,
    /// 操作取消
    Cancelled,
}

/// 配置错误类型
#[derive(Debug)]
pub enum ConfigError {
    /// 文件未找到
    NotFound(String),
    /// 解析失败
    ParseError(String),
    /// 无效值
    InvalidValue(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Api(e) => write!(f, "API request failed: {}", e),
            AppError::Config(e) => write!(f, "Configuration error: {}", e),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::KernelNotRunning => write!(f, "Mihomo kernel is not running"),
            AppError::KernelAlreadyRunning => write!(f, "Mihomo kernel is already running"),
            AppError::Cancelled => write!(f, "Operation cancelled"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse config: {}", msg),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid config value: {}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Api(e) => Some(e),
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Api(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e)
    }
}
