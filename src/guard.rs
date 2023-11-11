use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("posisoned resource")]
    Poisoned,
}

/// Tx represents a resource to be manipulated transactionally.
pub trait Tx<T> {
    type Guard<'a>: TxGuard<'a, T>
    where
        Self: 'a,
        T: 'a;

    /// Acquires the resource, blocking the current thread until it is available
    /// to do so.
    fn begin<'a>(&'a self) -> Result<Self::Guard<'a>, Error>;
}

/// A TxGuard holds a copy of T while keeping locked the original value, ensuring
/// its consistency between transactions.
pub trait TxGuard<'a, T>: Deref + DerefMut + AsRef<T> + AsMut<T> {
    /// Releases the resource right after updating its content with the
    /// manipulated data.
    fn commit(self);
}

pub struct Resource<T> {
    mu: Arc<Mutex<T>>,
}

impl<T> Tx<T> for Resource<T>
where
    T: Clone,
{
    type Guard<'a> = ResourceGuard<'a, T> where Self: 'a, T: 'a;

    fn begin<'a>(&'a self) -> Result<Self::Guard<'a>, Error> {
        let guard = self.mu.lock().map_err(|_| Error::Poisoned)?;
        Ok(ResourceGuard {
            data: guard.clone(),
            guard,
        })
    }
}

impl<T> From<Arc<Mutex<T>>> for Resource<T> {
    fn from(value: Arc<Mutex<T>>) -> Self {
        Self { mu: value }
    }
}

pub struct ResourceGuard<'a, T> {
    guard: MutexGuard<'a, T>,
    data: T,
}

impl<'a, T> Deref for ResourceGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for ResourceGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T> AsRef<T> for ResourceGuard<'a, T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<'a, T> AsMut<T> for ResourceGuard<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<'a, T> TxGuard<'a, T> for ResourceGuard<'a, T> {
    fn commit(mut self) {
        *self.guard = self.data;
    }
}
