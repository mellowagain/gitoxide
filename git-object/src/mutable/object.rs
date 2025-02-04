use std::io;

use crate::mutable;

/// A mutable object representing [`Trees`][mutable::Tree], [`Blobs`][mutable::Blob], [`Commits`][mutable::Commit] or [`Tags`][mutable::Tag].
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant, missing_docs)]
pub enum Object {
    Tree(mutable::Tree),
    Blob(mutable::Blob),
    Commit(mutable::Commit),
    Tag(mutable::Tag),
}

/// Convenient extraction of typed object.
impl Object {
    /// Returns a [`Blob`][mutable::Blob] if it is one.
    pub fn as_blob(&self) -> Option<&mutable::Blob> {
        match self {
            Object::Blob(v) => Some(v),
            _ => None,
        }
    }
    /// Returns a [`Commit`][mutable::Commit] if it is one.
    pub fn as_commit(&self) -> Option<&mutable::Commit> {
        match self {
            Object::Commit(v) => Some(v),
            _ => None,
        }
    }
    /// Returns a [`Tree`][mutable::Tree] if it is one.
    pub fn as_tree(&self) -> Option<&mutable::Tree> {
        match self {
            Object::Tree(v) => Some(v),
            _ => None,
        }
    }
    /// Returns a [`Tag`][mutable::Tag] if it is one.
    pub fn as_tag(&self) -> Option<&mutable::Tag> {
        match self {
            Object::Tag(v) => Some(v),
            _ => None,
        }
    }
    /// Returns the kind of object stored in this instance.
    pub fn kind(&self) -> crate::Kind {
        match self {
            Object::Tree(_) => crate::Kind::Tree,
            Object::Blob(_) => crate::Kind::Blob,
            Object::Commit(_) => crate::Kind::Commit,
            Object::Tag(_) => crate::Kind::Tag,
        }
    }
}

/// Serialization
impl Object {
    /// Write the contained object to `out` in the git serialization format.
    pub fn write_to(&self, out: impl io::Write) -> io::Result<()> {
        use Object::*;
        match self {
            Tree(v) => v.write_to(out),
            Blob(v) => v.write_to(out),
            Commit(v) => v.write_to(out),
            Tag(v) => v.write_to(out),
        }
    }
}

mod convert {
    use std::convert::TryFrom;

    use crate::mutable::{Blob, Commit, Object, Tag, Tree};

    impl From<Tag> for Object {
        fn from(v: Tag) -> Self {
            Object::Tag(v)
        }
    }

    impl From<Commit> for Object {
        fn from(v: Commit) -> Self {
            Object::Commit(v)
        }
    }

    impl From<Tree> for Object {
        fn from(v: Tree) -> Self {
            Object::Tree(v)
        }
    }

    impl From<Blob> for Object {
        fn from(v: Blob) -> Self {
            Object::Blob(v)
        }
    }

    impl TryFrom<Object> for Tag {
        type Error = Object;

        fn try_from(value: Object) -> Result<Self, Self::Error> {
            Ok(match value {
                Object::Tag(v) => v,
                _ => return Err(value),
            })
        }
    }

    impl TryFrom<Object> for Commit {
        type Error = Object;

        fn try_from(value: Object) -> Result<Self, Self::Error> {
            Ok(match value {
                Object::Commit(v) => v,
                _ => return Err(value),
            })
        }
    }

    impl TryFrom<Object> for Tree {
        type Error = Object;

        fn try_from(value: Object) -> Result<Self, Self::Error> {
            Ok(match value {
                Object::Tree(v) => v,
                _ => return Err(value),
            })
        }
    }

    impl TryFrom<Object> for Blob {
        type Error = Object;

        fn try_from(value: Object) -> Result<Self, Self::Error> {
            Ok(match value {
                Object::Blob(v) => v,
                _ => return Err(value),
            })
        }
    }
}
