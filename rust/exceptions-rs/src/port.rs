#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadphonesError {
    Recoverable {
        message: String,
        should_disconnect: bool,
    },
    Runtime(String),
}

impl HeadphonesError {
    pub fn recoverable(message: impl Into<String>, should_disconnect: bool) -> Self {
        Self::Recoverable {
            message: message.into(),
            should_disconnect,
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }

    pub fn should_disconnect(&self) -> bool {
        matches!(
            self,
            Self::Recoverable {
                should_disconnect: true,
                ..
            }
        )
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::Recoverable { .. })
    }
}

impl fmt::Display for HeadphonesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recoverable { message, .. } | Self::Runtime(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for HeadphonesError {}
