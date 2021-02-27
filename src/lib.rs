//! This library provides simple util for testing file system operations.
//! When you test something you need some sandbox which should wipe out after testing finish.
//! This package allow you configure temporary directory and its inner structure, perform tests and
//! remove it when all work will done.

mod rfs;

pub use rfs::error::FsTesterError;

pub use rfs::fs_tester::Config;
pub use rfs::fs_tester::ConfigEntry;
pub use rfs::fs_tester::DirectoryConf;
pub use rfs::fs_tester::FileConf;
pub use rfs::fs_tester::FileContent;
pub use rfs::fs_tester::FsTester;

