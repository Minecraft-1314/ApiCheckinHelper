use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkFault {
    pub message: String,
    pub kind: String,
    pub retryable: bool,
}

impl NetworkFault {
    pub fn new(message: impl Into<String>, kind: impl Into<String>, retryable: bool) -> Self {
        Self {
            message: message.into(),
            kind: kind.into(),
            retryable,
        }
    }

    pub fn timeout() -> Self {
        Self::new("请求超时", "timeout", true)
    }

    pub fn offline() -> Self {
        Self::new("无法连接到站点", "connect", true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Validation(String),
    Cancelled,
    Network(NetworkFault),
    General(String),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn general(message: impl Into<String>) -> Self {
        Self::General(message.into())
    }

    pub fn message(&self) -> String {
        match self {
            Self::Validation(text) => text.clone(),
            Self::Cancelled => "操作已取消".to_string(),
            Self::Network(fault) => fault.message.clone(),
            Self::General(text) => text.clone(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Validation(_) => "ConfigError",
            Self::Cancelled => "OperationCanceledException",
            Self::Network(_) => "NetError",
            Self::General(_) => "Exception",
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    pub fn fault(&self) -> Option<&NetworkFault> {
        match self {
            Self::Network(fault) => Some(fault),
            _ => None,
        }
    }

    pub fn fault_kind(&self) -> &str {
        match self {
            Self::Network(fault) => fault.kind.as_str(),
            _ => "",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
