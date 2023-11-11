use super::{service::EventRepository, Error, Event, Result};
use crate::{guard::Resource, id::Id, interval::Interval};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InMemoryEventRepository<I>
where
    I: Interval,
{
    events: RwLock<HashMap<Id<Event<I>>, Arc<Mutex<Event<I>>>>>,
}

impl<I> EventRepository for InMemoryEventRepository<I>
where
    I: Interval,
{
    type Interval = I;
    type Tx = Resource<Event<I>>;

    fn create(&self, event: &Event<I>) -> Result<()> {
        let mut events = self
            .events
            .write()
            .map_err(|err| Error::Lock(err.to_string()))?;

        if events.contains_key(&event.id) {
            return Err(Error::AlreadyExists);
        }

        events.insert(event.id, Arc::new(Mutex::new(event.clone())));
        Ok(())
    }

    fn find(&self, id: Id<Event<I>>) -> Result<Self::Tx> {
        let events = self
            .events
            .read()
            .map_err(|err| Error::Lock(err.to_string()))?;

        events
            .get(&id)
            .cloned()
            .ok_or(Error::NotFound)
            .map(Resource::from)
    }
}
