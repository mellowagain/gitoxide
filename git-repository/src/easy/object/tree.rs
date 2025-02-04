use crate::{
    easy,
    easy::{object::find, TreeRef},
    objs,
    objs::{bstr::BStr, immutable},
};

impl<'repo, A> TreeRef<'repo, A>
where
    A: easy::Access + Sized,
{
    // TODO: move implementation to git-object, tests.
    pub fn lookup_path<I, P>(mut self, path: I) -> Result<Option<objs::mutable::tree::Entry>, find::existing::Error>
    where
        I: IntoIterator<Item = P>,
        P: PartialEq<BStr>,
    {
        // let mut out = None;
        let mut path = path.into_iter().peekable();
        while let Some(component) = path.next() {
            match immutable::tree::TreeIter::from_bytes(&self.data)
                .filter_map(Result::ok)
                .find(|entry| component.eq(entry.filename))
            {
                Some(entry) => {
                    if path.peek().is_none() {
                        return Ok(Some(entry.into()));
                    } else {
                        let next_id = entry.oid.to_owned();
                        let access = self.access;
                        drop(entry);
                        drop(self);
                        self = match crate::easy::ext::object::find_object(access, next_id)?.try_into_tree() {
                            Ok(tree) => tree,
                            Err(_) => return Ok(None),
                        };
                    }
                }
                None => return Ok(None),
            }
        }
        Ok(None)
    }
}
