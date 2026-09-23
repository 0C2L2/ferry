//! The Windows → Ubuntu handoff files: written by the Windows app at backup
//! time, read by `ferry-restore` on Ubuntu. Portable so both sides share one
//! definition (and so the rules are unit-tested on the Windows build too).

pub mod apps;
pub mod bookmarks;
pub mod firefox;
pub mod settings;
