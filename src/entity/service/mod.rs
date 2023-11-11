mod create;
pub use create::*;

mod filter;
pub use filter::*;

mod find;
pub use find::*;

mod remove;
pub use remove::*;

use super::{error::Result, Entity};
use crate::{guard::Tx, id::Id};
use std::sync::Arc;

pub trait EntityRepository {
    type Tx: Tx<Entity>;

    fn find(&self, id: Id<Entity>) -> Result<Self::Tx>;
    fn filter(&self, filter: &EntityFilter) -> Result<Vec<Self::Tx>>;
    fn create(&self, entity: &Entity) -> Result<()>;
    fn delete(&self, id: Id<Entity>) -> Result<()>;
}

pub struct EntityService<R> {
    pub entity_repo: Arc<R>,
}
