use std::{cmp::Ordering, io};

use bstr::{BString, ByteSlice};
use quick_error::quick_error;

use crate::{mutable::SPACE, tree::EntryMode};

quick_error! {
    /// The Error used in [`Tree::write_to()`].
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        NewlineInFilename(name: BString) {
            display("Newlines are invalid in file paths: {:?}", name)
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

/// A mutable Tree, containing other trees, blobs or commits.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Tree {
    /// The directories and files contained in this tree. They must be and remain sorted by [`filename`][Entry::filename].
    pub entries: Vec<Entry>,
}

/// An entry in a [`Tree`], similar to an entry in a directory.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The kind of object to which `oid` is pointing to.
    pub mode: EntryMode,
    /// The name of the file in the parent tree.
    pub filename: BString,
    /// The id of the object representing the entry.
    pub oid: git_hash::ObjectId,
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    /// Entries compare by the common portion of the filename. This is critical for proper functioning of algorithms working on trees.
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.filename.len().min(other.filename.len());
        self.filename[..len].cmp(&other.filename[..len])
    }
}

/// Serialization
impl EntryMode {
    /// Return the representation as used in the git internal format.
    pub fn as_bytes(&self) -> &'static [u8] {
        use EntryMode::*;
        match self {
            Tree => b"40000",
            Blob => b"100644",
            BlobExecutable => b"100755",
            Link => b"120000",
            Commit => b"160000",
        }
    }
}

/// Serialization
impl Tree {
    /// Serialize this tree to `out` in the git internal format.
    pub fn write_to(&self, mut out: impl io::Write) -> io::Result<()> {
        debug_assert_eq!(
            &{
                let mut entries_sorted = self.entries.clone();
                entries_sorted.sort();
                entries_sorted
            },
            &self.entries,
            "entries for serialization must be sorted by filename"
        );
        for Entry { mode, filename, oid } in &self.entries {
            out.write_all(mode.as_bytes())?;
            out.write_all(SPACE)?;

            if filename.find_byte(b'\n').is_some() {
                return Err(Error::NewlineInFilename(filename.to_owned()).into());
            }
            out.write_all(filename)?;
            out.write_all(&[b'\0'])?;

            out.write_all(oid.as_bytes())?;
        }
        Ok(())
    }
}
