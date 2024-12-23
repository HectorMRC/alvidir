//! Identity definition.

/// An entity that can be uniquely identified.
pub trait Identify {
    type Id;

    fn id(&self) -> &Self::Id;
}

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures {
    use super::Identify;

    /// A mock implementation of the [`Identify`] trait.
    pub struct IndentifyMock<'a, Id> {
        id_fn: Option<fn() -> &'a Id>,
    }

    impl<'a, Id> Identify for IndentifyMock<'a, Id> {
        type Id = Id;

        fn id(&self) -> &Self::Id {
            self.id_fn.expect("id method must be set")()
        }
    }
}
