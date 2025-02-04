use std::path::PathBuf;

use crate::{
    alternate,
    store::{compound, linked},
};

/// The error returned by [`linked::Store::at()`]
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    CompoundDbInit(#[from] compound::init::Error),
    #[error(transparent)]
    AlternateResolve(#[from] alternate::Error),
}

impl linked::Store {
    /// Instantiate an instance at the given `objects_directory`, commonly `.git/objects`.
    ///
    /// _git alternate_ files will be traversed to build a chain of [`compound::Store`] instances.
    pub fn at(objects_directory: impl Into<PathBuf>) -> Result<Self, Error> {
        let mut dbs = vec![compound::Store::at(objects_directory.into())?];
        for object_path in alternate::resolve(dbs[0].loose.path.clone())?.into_iter() {
            dbs.push(compound::Store::at(object_path)?);
        }
        assert!(
            !dbs.is_empty(),
            "we can rely on at least one compound database to be present"
        );
        Ok(linked::Store { dbs })
    }
}

impl std::convert::TryFrom<PathBuf> for linked::Store {
    type Error = Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        linked::Store::at(value)
    }
}
