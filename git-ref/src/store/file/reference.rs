use std::convert::TryFrom;

use bstr::ByteSlice;
use git_hash::ObjectId;

use crate::{
    file::loose::reference::logiter::must_be_io_err,
    mutable,
    store::{
        file,
        file::{log, loose},
        packed,
    },
    FullName, Namespace,
};

/// Either a loose or packed reference, depending on where it was found.
#[derive(Debug)]
pub enum Reference<'p> {
    /// A reference originating in a pack
    Packed(packed::Reference<'p>),
    /// A reference from the filesystem
    Loose(loose::Reference),
}

impl<'p> TryFrom<Reference<'p>> for loose::Reference {
    type Error = ();

    fn try_from(value: Reference<'p>) -> Result<Self, Self::Error> {
        match value {
            Reference::Loose(l) => Ok(l),
            Reference::Packed(_) => Err(()),
        }
    }
}

impl<'p> TryFrom<Reference<'p>> for packed::Reference<'p> {
    type Error = ();

    fn try_from(value: Reference<'p>) -> Result<Self, Self::Error> {
        match value {
            Reference::Loose(_) => Err(()),
            Reference::Packed(p) => Ok(p),
        }
    }
}

impl<'p> Reference<'p> {
    /// For details, see [loose::Reference::log_exists()].
    pub fn log_exists(&self, store: &file::Store) -> bool {
        match self {
            Reference::Loose(r) => r.log_exists(store),
            Reference::Packed(p) => store.reflog_exists(p.name).expect("infallible name conversion"),
        }
    }

    /// For details, see [crate::file::loose::Reference::peel_to_id_in_place].
    pub fn peel_to_id_in_place<E: std::error::Error + Send + Sync + 'static>(
        &mut self,
        store: &file::Store,
        packed: Option<&packed::Buffer>,
        find: impl FnMut(git_hash::ObjectId, &mut Vec<u8>) -> Result<Option<(git_object::Kind, &[u8])>, E>,
    ) -> Result<ObjectId, crate::store::file::loose::reference::peel::to_id::Error> {
        match self {
            Reference::Loose(r) => r.peel_to_id_in_place(store, packed, find).map(ToOwned::to_owned),
            Reference::Packed(p) => {
                if let Some(object) = p.object {
                    p.target = object;
                }
                p.object = None;
                Ok(p.target())
            }
        }
    }

    /// For details, see [crate::file::loose::Reference::follow_symbolic()].
    pub fn peel_one_level<'p2>(
        &self,
        store: &file::Store,
        packed: Option<&'p2 packed::Buffer>,
    ) -> Option<Result<Reference<'p2>, crate::store::file::loose::reference::peel::Error>> {
        match self {
            Reference::Loose(r) => r.follow_symbolic(store, packed),
            Reference::Packed(p) => packed
                .and_then(|packed| packed.find(p.name).ok().flatten()) // needed to get data with 'p2 lifetime
                .and_then(|np| {
                    p.object.and(np.object).map(|peeled| {
                        Ok(Reference::Packed(packed::Reference {
                            name: np.name,
                            target: peeled,
                            object: None,
                        }))
                    })
                }),
        }
    }

    /// Obtain a reverse iterator over logs of this reference. See [crate::file::loose::Reference::log_iter_rev()] for details.
    pub fn log_iter_rev<'b>(
        &self,
        store: &file::Store,
        buf: &'b mut [u8],
    ) -> std::io::Result<Option<log::iter::Reverse<'b, std::fs::File>>> {
        match self {
            Reference::Loose(r) => r.log_iter_rev(store, buf),
            Reference::Packed(p) => store.reflog_iter_rev(p.name, buf).map_err(must_be_io_err),
        }
    }

    /// Obtain an iterator over logs of this reference. See [crate::file::loose::Reference::log_iter()] for details.
    pub fn log_iter<'a, 'b: 'a>(
        &'a self,
        store: &file::Store,
        buf: &'b mut Vec<u8>,
    ) -> std::io::Result<Option<impl Iterator<Item = Result<log::Line<'b>, log::iter::decode::Error>> + 'a>> {
        match self {
            Reference::Loose(r) => store.reflog_iter(r.name.borrow(), buf).map_err(must_be_io_err),
            Reference::Packed(p) => store.reflog_iter(p.name, buf).map_err(must_be_io_err),
        }
    }

    /// Returns the kind of reference
    pub fn kind(&self) -> crate::Kind {
        match self {
            Reference::Loose(r) => r.kind(),
            Reference::Packed(_) => crate::Kind::Peeled,
        }
    }

    /// Transform this reference into an owned `Target`
    pub fn into_target(self) -> mutable::Target {
        match self {
            Reference::Packed(p) => mutable::Target::Peeled(p.object()),
            Reference::Loose(r) => r.target,
        }
    }

    /// Returns true if this ref is located in a packed ref buffer.
    pub fn is_packed(&self) -> bool {
        match self {
            Reference::Packed(_) => true,
            Reference::Loose(_) => false,
        }
    }

    /// Return the full validated name of the reference, which may include a namespace.
    pub fn name(&self) -> FullName<'_> {
        match self {
            Reference::Packed(p) => p.name,
            Reference::Loose(l) => l.name.borrow(),
        }
    }

    /// Return the full validated name of the reference, with the given namespace stripped if possible.
    ///
    /// If the reference name wasn't prefixed with `namespace`, `None` is returned instead.
    pub fn name_without_namespace(&self, namespace: &Namespace) -> Option<FullName<'_>> {
        self.name()
            .0
            .as_bstr()
            .strip_prefix(namespace.0.as_bstr().as_ref())
            .map(|stripped| FullName(stripped.as_bstr()))
    }

    /// Return the target to which the reference points to.
    pub fn target(&self) -> mutable::Target {
        match self {
            Reference::Packed(p) => mutable::Target::Peeled(p.target()),
            Reference::Loose(l) => l.target.clone(),
        }
    }
}
