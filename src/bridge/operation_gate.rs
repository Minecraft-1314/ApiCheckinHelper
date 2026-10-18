use std::sync::Mutex;

use crate::core::cancel::CancelToken;
use crate::core::error::{AppError, AppResult};

pub struct OperationGate {
    current: Mutex<Option<CancelToken>>,
}

impl OperationGate {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    pub fn create(&self) -> AppResult<CancelToken> {
        let mut guard = self.lock();
        if let Some(current) = guard.as_ref() {
            if !current.is_cancelled() {
                return Err(AppError::validation("已有任务正在运行"));
            }
        }
        let lease = CancelToken::new();
        *guard = Some(lease.clone());
        Ok(lease)
    }

    pub fn release(&self, lease: &CancelToken) {
        let mut guard = self.lock();
        let matched = guard
            .as_ref()
            .map(|current| current.same_as(lease))
            .unwrap_or(false);
        if matched {
            *guard = None;
        }
    }

    pub fn cancel_current(&self) {
        if let Some(current) = self.lock().as_ref() {
            current.cancel();
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<CancelToken>> {
        self.current
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

impl Default for OperationGate {
    fn default() -> Self {
        Self::new()
    }
}
