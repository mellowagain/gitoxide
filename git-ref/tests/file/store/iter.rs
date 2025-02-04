use std::convert::TryInto;

use bstr::ByteSlice;
use git_testtools::hex_to_id;

use crate::file::{store, store_at, store_with_packed_refs};

mod with_namespace {
    use bstr::BString;
    use git_object::bstr::ByteSlice;

    use crate::file::store_at;

    #[test]
    fn general_iteration_can_trivially_use_namespaces_as_prefixes() {
        let store = store_at("make_namespaced_packed_ref_repository.sh").unwrap();
        let packed = store.packed_buffer().unwrap();

        let ns_two = git_ref::namespace::expand("bar").unwrap();
        assert_eq!(
            store
                .iter_prefixed(packed.as_ref(), ns_two.to_path())
                .unwrap()
                .map(Result::unwrap)
                .map(|r: git_ref::file::Reference| r.name().as_bstr().to_owned())
                .collect::<Vec<_>>(),
            vec![
                "refs/namespaces/bar/refs/heads/multi-link-target1",
                "refs/namespaces/bar/refs/multi-link",
                "refs/namespaces/bar/refs/remotes/origin/multi-link-target3",
                "refs/namespaces/bar/refs/tags/multi-link-target2"
            ]
        );

        let ns_one = git_ref::namespace::expand("foo").unwrap();
        assert_eq!(
            store
                .iter_prefixed(packed.as_ref(), ns_one.to_path())
                .unwrap()
                .map(Result::unwrap)
                .map(|r: git_ref::file::Reference| (
                    r.name().as_bstr().to_owned(),
                    r.name_without_namespace(&ns_one)
                        .expect("stripping correct namespace always works")
                        .as_bstr()
                        .to_owned()
                ))
                .collect::<Vec<_>>(),
            vec![
                (BString::from("refs/namespaces/foo/refs/d1"), BString::from("refs/d1")),
                (
                    "refs/namespaces/foo/refs/remotes/origin/HEAD".into(),
                    "refs/remotes/origin/HEAD".into()
                ),
                (
                    "refs/namespaces/foo/refs/remotes/origin/main".into(),
                    "refs/remotes/origin/main".into()
                )
            ]
        );

        assert_eq!(
            store
                .iter(packed.as_ref())
                .unwrap()
                .map(Result::unwrap)
                .filter_map(
                    |r: git_ref::file::Reference| if r.name().as_bstr().starts_with_str("refs/namespaces") {
                        None
                    } else {
                        Some(r.name().as_bstr().to_owned())
                    }
                )
                .collect::<Vec<_>>(),
            vec![
                "refs/heads/d1",
                "refs/heads/dt1",
                "refs/heads/main",
                "refs/tags/dt1",
                "refs/tags/t1"
            ],
            "we can find refs without namespace by manual filter, really just for testing purposes"
        );
    }
}

#[test]
fn no_packed_available_thus_no_iteration_possible() -> crate::Result {
    let store_without_packed = store()?;
    assert!(
        store_without_packed.packed_buffer()?.is_none(),
        "there is no packed refs in this store"
    );
    Ok(())
}

#[test]
fn packed_file_iter() -> crate::Result {
    let store = store_with_packed_refs()?;
    assert_eq!(store.packed_buffer()?.expect("pack available").iter()?.count(), 8);
    Ok(())
}

#[test]
fn loose_iter_with_broken_refs() -> crate::Result {
    let store = store()?;

    let mut actual: Vec<_> = store.loose_iter()?.collect();
    assert_eq!(actual.len(), 15);
    actual.sort_by_key(|r| r.is_err());
    let first_error = actual
        .iter()
        .enumerate()
        .find_map(|(idx, r)| if r.is_err() { Some(idx) } else { None })
        .expect("there is an error");

    assert_eq!(
        first_error, 14,
        "there is exactly one invalid item, and it didn't abort the iterator most importantly"
    );
    #[cfg(not(windows))]
    let msg = "The reference at 'refs/broken' could not be instantiated";
    #[cfg(windows)]
    let msg = "The reference at 'refs\\broken' could not be instantiated";
    assert_eq!(
        actual[first_error].as_ref().expect_err("unparseable ref").to_string(),
        msg
    );
    let ref_paths: Vec<_> = actual
        .drain(..first_error)
        .filter_map(|e| e.ok().map(|e| e.name.into_inner()))
        .collect();

    assert_eq!(
        ref_paths,
        vec![
            "d1",
            "heads/d1",
            "heads/dt1",
            "heads/main",
            "heads/multi-link-target1",
            "loop-a",
            "loop-b",
            "multi-link",
            "remotes/origin/HEAD",
            "remotes/origin/main",
            "remotes/origin/multi-link-target3",
            "tags/dt1",
            "tags/multi-link-target2",
            "tags/t1"
        ]
        .into_iter()
        .map(|p| format!("refs/{}", p))
        .collect::<Vec<_>>(),
        "all paths are as expected"
    );
    Ok(())
}

#[test]
fn loose_iter_with_prefix_wont_allow_absolute_paths() -> crate::Result {
    let store = store()?;
    #[cfg(not(windows))]
    let abs_path = "/hello";
    #[cfg(windows)]
    let abs_path = "c:\\hello";

    match store.loose_iter_prefixed(abs_path) {
        Ok(_) => unreachable!("absolute paths aren't allowed"),
        Err(err) => assert_eq!(err.to_string(), "prefix must be a relative path, like 'refs/heads'"),
    }
    Ok(())
}

#[test]
fn loose_iter_with_prefix() -> crate::Result {
    let store = store()?;

    let actual = store
        .loose_iter_prefixed("refs/heads/")?
        .collect::<Result<Vec<_>, _>>()
        .expect("no broken ref in this subset")
        .into_iter()
        .map(|e| e.name.into_inner())
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        vec![
            "refs/heads/d1",
            "refs/heads/dt1",
            "refs/heads/main",
            "refs/heads/multi-link-target1",
        ]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>(),
        "all paths are as expected"
    );
    Ok(())
}

#[test]
fn overlay_iter() -> crate::Result {
    use git_ref::mutable::Target::*;

    let store = store_at("make_packed_ref_repository_for_overlay.sh")?;
    let ref_names = store
        .iter(store.packed_buffer()?.as_ref())?
        .map(|r| r.map(|r| (r.name().as_bstr().to_owned(), r.target(), r.is_packed())))
        .collect::<Result<Vec<_>, _>>()?;
    let c1 = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let c2 = hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7");
    assert_eq!(
        ref_names,
        vec![
            (b"refs/heads/main".as_bstr().to_owned(), Peeled(c1), true),
            ("refs/heads/newer-as-loose".into(), Peeled(c2), false),
            (
                "refs/remotes/origin/HEAD".into(),
                Symbolic("refs/remotes/origin/main".try_into()?),
                false
            ),
            ("refs/remotes/origin/main".into(), Peeled(c1), true),
            (
                "refs/tags/tag-object".into(),
                Peeled(hex_to_id("b3109a7e51fc593f85b145a76c70ddd1d133fafd")),
                true
            )
        ]
    );
    Ok(())
}

#[test]
fn overlay_iter_with_prefix_wont_allow_absolute_paths() -> crate::Result {
    let store = store_with_packed_refs()?;
    #[cfg(not(windows))]
    let abs_path = "/hello";
    #[cfg(windows)]
    let abs_path = "c:\\hello";

    match store.iter_prefixed(store.packed_buffer()?.as_ref(), abs_path) {
        Ok(_) => unreachable!("absolute paths aren't allowed"),
        Err(err) => assert_eq!(err.to_string(), "prefix must be a relative path, like 'refs/heads'"),
    }
    Ok(())
}

#[test]
fn overlay_prefixed_iter() -> crate::Result {
    use git_ref::mutable::Target::*;

    let store = store_at("make_packed_ref_repository_for_overlay.sh")?;
    let ref_names = store
        .iter_prefixed(store.packed_buffer()?.as_ref(), "refs/heads")?
        .map(|r| r.map(|r| (r.name().as_bstr().to_owned(), r.target(), r.is_packed())))
        .collect::<Result<Vec<_>, _>>()?;
    let c1 = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let c2 = hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7");
    assert_eq!(
        ref_names,
        vec![
            (b"refs/heads/main".as_bstr().to_owned(), Peeled(c1), true),
            ("refs/heads/newer-as-loose".into(), Peeled(c2), false),
        ]
    );
    Ok(())
}
