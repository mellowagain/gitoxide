use crate::file::{
    store_writable,
    transaction::prepare_and_commit::{committer, empty_store},
};
use git_lock::acquire::Fail;
use git_ref::{
    mutable::Target,
    transaction::{Change, RefEdit, RefLog},
};
use git_testtools::hex_to_id;
use std::convert::TryInto;

#[test]
fn delete_a_ref_which_is_gone_succeeds() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: None,
                    log: RefLog::AndReference,
                },
                name: "DOES_NOT_EXIST".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
        )?
        .commit(&committer())?;
    assert_eq!(edits.len(), 1);
    Ok(())
}

#[test]
fn delete_a_ref_which_is_gone_but_must_exist_fails() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                previous: Some(Target::must_exist()),
                log: RefLog::AndReference,
            },
            name: "DOES_NOT_EXIST".try_into()?,
            deref: false,
        }),
        Fail::Immediately,
    );
    match res {
        Ok(_) => unreachable!("must exist, but it doesn't actually exist"),
        Err(err) => assert_eq!(
            err.to_string(),
            "The reference 'DOES_NOT_EXIST' for deletion did not exist or could not be parsed"
        ),
    }
    Ok(())
}

#[test]
fn delete_ref_and_reflog_on_symbolic_no_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.loose_find_existing("HEAD")?;
    assert!(head.log_exists(&store));
    let _main = store.loose_find_existing("main")?;

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: Some(Target::must_exist()),
                    log: RefLog::AndReference,
                },
                name: head.name.clone(),
                deref: false,
            }),
            Fail::Immediately,
        )?
        .commit(&committer())?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Delete {
                previous: Some(Target::Symbolic("refs/heads/main".try_into()?)),
                log: RefLog::AndReference,
            },
            name: head.name,
            deref: false
        }],
        "the previous value was updated with the actual one"
    );
    assert!(
        store.reflog_iter_rev("HEAD", &mut [0u8; 128])?.is_none(),
        "reflog was deleted"
    );
    assert!(store.loose_find("HEAD")?.is_none(), "ref was deleted");
    assert!(store.loose_find("main")?.is_some(), "referent still exists");
    Ok(())
}

#[test]
fn delete_ref_with_incorrect_previous_value_fails() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.loose_find_existing("HEAD")?;
    assert!(head.log_exists(&store));

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                previous: Some(Target::Symbolic("refs/heads/main".try_into()?)),
                log: RefLog::Only,
            },
            name: head.name,
            deref: true,
        }),
        Fail::Immediately,
    );

    match res {
        Err(err) => {
            assert_eq!(err.to_string(), "The reference 'refs/heads/main' should have content ref: refs/heads/main, actual content was 02a7a22d90d7c02fb494ed25551850b868e634f0");
        }
        Ok(_) => unreachable!("must be err"),
    }
    // everything stays as is
    let head = store.loose_find_existing("HEAD")?;
    assert!(head.log_exists(&store));
    let main = store.loose_find_existing("main").expect("referent still exists");
    assert!(main.log_exists(&store));
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_no_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.loose_find_existing("HEAD")?;
    assert!(head.log_exists(&store));

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: Some(Target::Symbolic("refs/heads/main".try_into()?)),
                    log: RefLog::Only,
                },
                name: head.name,
                deref: false,
            }),
            Fail::Immediately,
        )?
        .commit(&committer())?;

    assert_eq!(edits.len(), 1);
    let head = store.loose_find_existing("HEAD")?;
    assert!(!head.log_exists(&store));
    let main = store.loose_find_existing("main").expect("referent still exists");
    assert!(main.log_exists(&store), "log is untouched, too");
    assert_eq!(
        main.target,
        head.follow_symbolic(&store, None).expect("a symref")?.target(),
        "head points to main"
    );
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_with_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.loose_find_existing("HEAD")?;
    assert!(head.log_exists(&store));

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: Some(Target::must_exist()),
                    log: RefLog::Only,
                },
                name: head.name,
                deref: true,
            }),
            Fail::Immediately,
        )?
        .commit(&committer())?;

    assert_eq!(edits.len(), 2);
    let head = store.loose_find_existing("HEAD")?;
    assert!(!head.log_exists(&store));
    let main = store.loose_find_existing("main").expect("referent still exists");
    assert!(!main.log_exists(&store), "log is removed");
    assert_eq!(
        main.target,
        head.follow_symbolic(&store, None).expect("a symref")?.target(),
        "head points to main"
    );
    Ok(())
}

#[test]
/// Based on https://github.com/git/git/blob/master/refs/files-backend.c#L514:L515
fn delete_broken_ref_that_must_exist_fails_as_it_is_no_valid_ref() -> crate::Result {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.base.join("HEAD"), &b"broken")?;
    assert!(store.loose_find("HEAD").is_err(), "the ref is truly broken");

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                previous: Some(Target::must_exist()),
                log: RefLog::AndReference,
            },
            name: "HEAD".try_into()?,
            deref: true,
        }),
        Fail::Immediately,
    );
    match res {
        Err(err) => {
            assert_eq!(
                err.to_string(),
                "The reference 'HEAD' for deletion did not exist or could not be parsed"
            );
        }
        Ok(_) => unreachable!("expected error"),
    }
    Ok(())
}

#[test]
/// Based on https://github.com/git/git/blob/master/refs/files-backend.c#L514:L515
fn delete_broken_ref_that_may_not_exist_works_even_in_deref_mode() -> crate::Result {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.base.join("HEAD"), &b"broken")?;
    assert!(store.loose_find("HEAD").is_err(), "the ref is truly broken");

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: None,
                    log: RefLog::AndReference,
                },
                name: "HEAD".try_into()?,
                deref: true,
            }),
            Fail::Immediately,
        )?
        .commit(&committer())?;

    assert!(store.loose_find("HEAD")?.is_none(), "the ref was deleted");
    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Delete {
                previous: None,
                log: RefLog::AndReference,
            },
            name: "HEAD".try_into()?,
            deref: false,
        }]
    );
    Ok(())
}

#[test]
fn store_write_mode_has_no_effect_and_reflogs_are_always_deleted() -> crate::Result {
    for reflog_writemode in &[git_ref::file::WriteReflog::Normal, git_ref::file::WriteReflog::Disable] {
        let (_keep, mut store) = store_writable("make_repo_for_reflog.sh")?;
        store.write_reflog = *reflog_writemode;
        assert!(store.loose_find_existing("HEAD")?.log_exists(&store));
        assert!(store.packed_buffer()?.is_none(), "there is no pack");

        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Delete {
                        previous: None,
                        log: RefLog::Only,
                    },
                    name: "HEAD".try_into()?,
                    deref: false,
                }),
                Fail::Immediately,
            )?
            .commit(&committer())?;
        assert_eq!(edits.len(), 1);
        assert!(
            !store.loose_find_existing("HEAD")?.log_exists(&store),
            "log was deleted"
        );
        assert!(store.packed_buffer()?.is_none(), "there still is no pack");
    }
    Ok(())
}

#[test]
fn packed_refs_are_consulted_when_determining_previous_value_of_ref_to_be_deleted_and_are_deleted_from_packed_ref_file(
) -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
    assert!(
        store.loose_find("main")?.is_none(),
        "no loose main available, it's packed"
    );
    assert!(
        store.packed_buffer()?.expect("packed").find("main")?.is_some(),
        "packed main is available"
    );

    let old_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: Some(Target::Peeled(old_id)),
                    log: RefLog::AndReference,
                },
                name: "refs/heads/main".try_into()?,
                deref: false,
            }),
            git_lock::acquire::Fail::Immediately,
        )?
        .commit(&committer())?;

    assert_eq!(edits.len(), 1, "an edit was performed in the packed refs store");
    let packed = store.packed_buffer()?.expect("packed ref present");
    assert!(packed.find("main")?.is_none(), "no main present after deletion");
    Ok(())
}

#[test]
fn a_loose_ref_with_old_value_check_and_outdated_packed_refs_value_deletes_both_refs() -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository_for_overlay.sh")?;
    let packed = store.packed_buffer()?.expect("packed-refs");
    let branch = store.find_existing("newer-as-loose", Some(&packed))?;
    let branch_id = branch.target().as_id().map(ToOwned::to_owned).expect("peeled");
    assert_ne!(
        packed.find_existing("newer-as-loose")?.target(),
        branch_id,
        "the packed ref is outdated"
    );

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    previous: Some(Target::Peeled(branch_id)),
                    log: RefLog::AndReference,
                },
                name: branch.name().into(),
                deref: false,
            }),
            git_lock::acquire::Fail::Immediately,
        )?
        .commit(&committer())?;

    assert_eq!(
        edits.len(),
        1,
        "only one edit even though technically two places were changed"
    );
    assert!(
        store.find("newer-as-loose", store.packed_buffer()?.as_ref())?.is_none(),
        "reference is deleted everywhere"
    );
    Ok(())
}

#[test]
fn all_contained_references_deletes_the_packed_ref_file_too() {
    let (_keep, store) = store_writable("make_packed_ref_repository.sh").unwrap();

    let edits = store
        .transaction()
        .prepare(
            store
                .packed_buffer()
                .unwrap()
                .expect("packed-refs")
                .iter()
                .unwrap()
                .map(|r| {
                    let r = r.expect("valid ref");
                    RefEdit {
                        change: Change::Delete {
                            previous: Target::Peeled(r.target()).into(),
                            log: RefLog::AndReference,
                        },
                        name: r.name.into(),
                        deref: false,
                    }
                }),
            git_lock::acquire::Fail::Immediately,
        )
        .unwrap()
        .commit(&committer())
        .unwrap();

    assert!(!store.packed_refs_path().is_file(), "packed-refs was entirely removed");

    let packed = store.packed_buffer().unwrap();
    assert!(packed.is_none(), "it won't make up packed refs");
    for edit in edits {
        assert!(
            store.find(edit.name.to_partial(), packed.as_ref()).unwrap().is_none(),
            "delete ref cannot be found"
        );
    }
}
