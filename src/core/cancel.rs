use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use super::error::{AppError, AppResult};

#[derive(Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    children: Arc<Mutex<Vec<Weak<AtomicBool>>>>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            children: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
        if let Ok(children) = self.children.lock() {
            for child in children.iter() {
                if let Some(flag) = child.upgrade() {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn throw_if_cancelled(&self) -> AppResult<()> {
        if self.is_cancelled() {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }

    pub fn link(parents: &[&CancelToken]) -> CancelToken {
        let token = CancelToken::new();
        for parent in parents {
            if let Ok(mut children) = parent.children.lock() {
                children.push(Arc::downgrade(&token.flag));
            }
            if parent.is_cancelled() {
                token.cancel();
            }
        }
        token
    }

    pub fn child(&self) -> CancelToken {
        CancelToken::link(&[self])
    }

    pub fn same_as(&self, other: &CancelToken) -> bool {
        Arc::ptr_eq(&self.flag, &other.flag)
    }
}
