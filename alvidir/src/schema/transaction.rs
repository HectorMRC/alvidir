//! Transaction definition.

use std::sync::{Arc, OnceLock, RwLock};

use crate::{
    deref::{TryDeref, TryDerefMut},
    graph::{Graph, NodeProxy, Source},
    id::Identify,
};

use super::{guard::SchemaWriteGuard, resource::ResourceSet, trigger::TriggerSet, Schema};

/// Represents a set of operations that must be perfomed as a whole.
pub trait Transaction {
    type Target: Identify;

    /// Begins the transaction.
    fn begin(&self) -> Context<'_, Self::Target>;
    /// Commits the transaction.
    fn commit(self);
}

/// Represents an operation into the schema.
enum Operation<T>
where
    T: Identify,
{
    Save(T),
    Delete(T::Id),
}

impl<T> Identify for Operation<T>
where
    T: Identify,
{
    type Id = T::Id;

    fn id(&self) -> &Self::Id {
        match self {
            Operation::Save(node) => node.id(),
            Operation::Delete(node_id) => node_id,
        }
    }
}

/// Represents a subset of operations from a transaction.
pub struct Context<'a, T>
where
    T: Identify,
{
    graph: &'a Graph<T>,
    schema: &'a Schema<T>,
    operations: Arc<RwLock<Vec<Operation<T>>>>,
    target: Option<T>,
}

impl<T> TryDeref for Context<'_, T>
where
    T: Identify,
{
    type Target = T;

    fn try_deref(&self) -> Option<&Self::Target> {
        self.target.as_ref()
    }
}

impl<T> TryDerefMut for Context<'_, T>
where
    T: Identify,
{
    fn try_deref_mut(&mut self) -> Option<&mut Self::Target> {
        self.target.as_mut()
    }
}

impl<T> Source for Context<'_, T>
where
    T: Identify + Clone,
    T::Id: Ord + PartialEq,
{
    type Node = T;

    fn get(&self, id: &<Self::Node as Identify>::Id) -> Option<Self::Node> {
        let guard = match self.operations.read() {
            Ok(ops) => ops,
            Err(err) => err.into_inner(),
        };

        match guard.iter().rev().find(|&op| op.id() == id) {
            Some(Operation::Save(node)) => Some(node.clone()),
            Some(Operation::Delete(_)) => None,
            None => self.graph.get(id),
        }
    }

    fn contains(&self, id: &<Self::Node as Identify>::Id) -> bool {
        let guard = match self.operations.read() {
            Ok(ops) => ops,
            Err(err) => err.into_inner(),
        };

        match guard.iter().rev().find(|&op| op.id() == id) {
            Some(Operation::Save(_)) => true,
            Some(Operation::Delete(_)) => false,
            None => self.graph.contains(id),
        }
    }
}

impl<T> Context<'_, T>
where
    T: Identify + Clone,
    T::Id: Ord,
{
    /// Returns the [`NodeProxy`] for the given id.
    pub fn node(&self, node_id: T::Id) -> NodeProxy<'_, Self> {
        NodeProxy::new(self, node_id)
    }
}

impl<T> Context<'_, T>
where
    T: Identify,
{
    /// Assigns a target to this context.
    pub fn with_target(mut self, target: T) -> Self {
        self.target = Some(target);
        self
    }

    /// Registers the save operation as part of the transaction.
    pub fn save(&self, node: T) {
        let mut guard = match self.operations.write() {
            Ok(ops) => ops,
            Err(err) => err.into_inner(),
        };

        guard.push(Operation::Save(node));
    }

    /// Registers the delete operation as part of the transaction.
    pub fn delete(&self, node_id: T::Id) {
        let mut guard = match self.operations.write() {
            Ok(ops) => ops,
            Err(err) => err.into_inner(),
        };

        guard.push(Operation::Delete(node_id));
    }

    /// Returns a reference to the underlying schema's [`ResourceSet`].
    pub fn resources(&self) -> &ResourceSet {
        self.schema.resources()
    }

    /// Returns a reference to the underlying schema's [`TriggerSet`].
    pub fn triggers(&self) -> &TriggerSet<T> {
        self.schema.triggers()
    }

    /// Returns a new transaction holded by this context.
    #[inline]
    pub fn transaction(&self) -> Foreground<'_, T> {
        self.into()
    }
}

/// Represents a set of operations that must be completed transactionally.
pub struct Background<'a, T>
where
    T: Identify,
{
    schema: &'a Schema<T>,
    guard: OnceLock<SchemaWriteGuard<'a, T>>,
    operations: Arc<RwLock<Vec<Operation<T>>>>,
}

impl<'a, T> From<&'a Schema<T>> for Background<'a, T>
where
    T: Identify,
{
    fn from(schema: &'a Schema<T>) -> Self {
        Self {
            schema,
            guard: Default::default(),
            operations: Default::default(),
        }
    }
}

impl<T> Transaction for Background<'_, T>
where
    T: Identify,
    T::Id: Clone + Ord,
{
    type Target = T;

    fn begin(&self) -> Context<'_, T> {
        Context {
            schema: self.schema,
            graph: self.guard.get_or_init(|| self.schema.write()),
            operations: self.operations.clone(),
            target: Default::default(),
        }
    }

    fn commit(mut self) {
        let Some(mut guard) = self.guard.take() else {
            tracing::error!("committing uninitialized transaction");
            return;
        };

        let Some(ops) = Arc::into_inner(self.operations) else {
            tracing::error!("commiting transaction with contexts yet in use");
            return;
        };

        let ops = match ops.into_inner() {
            Ok(ops) => ops,
            Err(err) => {
                tracing::error!(error = err.to_string(), "committing poisoned transaction");
                return;
            }
        };

        let _ = ops.into_iter().filter_map(|op| match op {
            Operation::Save(node) => guard.insert(node),
            Operation::Delete(node_id) => guard.remove(&node_id),
        });
    }
}

/// Represents a subset of operations that must be completed transactionally.
pub struct Foreground<'a, T>
where
    T: Identify,
{
    context: &'a Context<'a, T>,
    operations: Arc<RwLock<Vec<Operation<T>>>>,
}

impl<'a, T> From<&'a Context<'a, T>> for Foreground<'a, T>
where
    T: Identify,
{
    fn from(context: &'a Context<'_, T>) -> Self {
        Foreground {
            context,
            operations: Default::default(),
        }
    }
}

impl<T> Transaction for Foreground<'_, T>
where
    T: Identify,
{
    type Target = T;

    fn begin(&self) -> Context<'_, T> {
        Context {
            graph: self.context.graph,
            schema: self.context.schema,
            operations: self.operations.clone(),
            target: Default::default(),
        }
    }

    fn commit(self) {
        let Some(ops) = Arc::into_inner(self.operations) else {
            tracing::error!("commiting transaction with contexts yet in use");
            return;
        };

        let ops = match ops.into_inner() {
            Ok(ops) => ops,
            Err(err) => {
                tracing::error!(error = err.to_string(), "committing poisoned transaction");
                return;
            }
        };

        let mut upstream_ops = match self.context.operations.write() {
            Ok(ops) => ops,
            Err(err) => {
                tracing::error!(
                    error = err.to_string(),
                    "committing transaction into poisoned context"
                );
                return;
            }
        };

        upstream_ops.extend(ops);
    }
}
