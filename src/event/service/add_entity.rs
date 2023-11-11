use super::{EventRepository, EventService};
use crate::{
    entity::{service::EntityRepository, Entity},
    event::{Event, Result},
    guard::{Tx, TxGuard},
    id::{Id, Identified},
};
use std::sync::Arc;

/// Implements the add entity transaction for an event.
pub struct AddEntity<R, E>
where
    R: EventRepository,
{
    event_repo: Arc<R>,
    entity_repo: Arc<E>,
    entity_id: Id<Entity>,
    event_id: Id<Event<R::Interval>>,
}

impl<R, E> AddEntity<R, E>
where
    R: EventRepository,
    E: EntityRepository,
{
    /// Executes the add entity transation.
    pub fn execute(self) -> Result<()> {
        let entity = self.entity_repo.find(&self.entity_id)?;
        let event_tx = self.event_repo.find(self.event_id)?;
        let mut event = event_tx.begin()?;

        event.as_mut().entities.push(entity.id());

        event.commit();
        Ok(())
    }
}

impl<R, E> EventService<R, E>
where
    R: EventRepository,
    E: EntityRepository,
{
    pub fn add_entity(
        &self,
        entity_id: Id<Entity>,
        event_id: Id<Event<R::Interval>>,
    ) -> AddEntity<R, E> {
        AddEntity {
            entity_repo: self.entity_repo.clone(),
            event_repo: self.event_repo.clone(),
            entity_id,
            event_id,
        }
    }
}