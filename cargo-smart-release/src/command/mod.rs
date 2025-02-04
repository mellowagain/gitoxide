pub mod release {
    #[derive(Debug, Clone, Copy)]
    pub struct Options {
        pub dry_run: bool,
        pub allow_dirty: bool,
        pub ignore_instability: bool,
        pub skip_publish: bool,
        pub dry_run_cargo_publish: bool,
        /// Pass --no-verify unconditionally to cargo publish. Really just for fixing things
        pub no_verify: bool,
        pub skip_tag: bool,
        pub allow_auto_publish_of_stable_crates: bool,
        pub update_crates_index: bool,
        pub no_bump_on_demand: bool,
        pub verbose: bool,
        pub skip_push: bool,
        pub skip_dependencies: bool,
    }
}
#[path = "release/mod.rs"]
mod release_impl;
pub use release_impl::release;
