use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use error::MasqueError;

type StoreResult<T> = Result<T, MasqueError>;

#[derive(Clone, Debug)]
pub struct Store<T> {
    data: Arc<RwLock<T>>,
}

impl<T> Store<T>
where
    T: Clone,
{
    pub fn new<S>(data: S) -> Store<T>
    where
        S: Into<T>,
    {
        Store {
            data: Arc::new(RwLock::new(data.into())),
        }
    }

    fn reader(&self) -> StoreResult<RwLockReadGuard<T>> {
        self.data.read().map_err(|_| {
            error!("Failed to acquire read guard for cache failed due to poisoning");
            MasqueError::StorePoisonedError
        })
    }

    fn writer(&self) -> StoreResult<RwLockWriteGuard<T>> {
        self.data.write().map_err(|_| {
            error!("Failed to acquire read guard for cache failed due to poisoning");
            MasqueError::StorePoisonedError
        })
    }

    pub fn get(&self) -> StoreResult<T> {
        self.reader().map(|guard| guard.clone())
    }

    pub fn update<S>(&self, data: S) -> StoreResult<T>
    where
        S: Into<T>,
    {
        self.writer().map(|mut guard| {
            *guard = data.into();
            guard.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_returns_raw_data() {
        let store: Store<String> = Store::new("initial");
        assert_eq!(store.get().unwrap(), "initial".to_string());
    }

    #[test]
    fn test_store_updates() {
        let store: Store<String> = Store::new("initial");
        let _ = store.update("updated");
        assert_eq!(store.get().unwrap(), "updated".to_string());
    }
}
