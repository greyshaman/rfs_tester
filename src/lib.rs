//! This library provides a simple utility for testing file system operations.
//! When you are testing something, you need a sandbox that can be wiped out after the testing is finished.
//! This package allows you to configure a temporary directory and its internal structure, run tests,
//! and remove it once all work is done.

pub mod rfs;

pub use rfs::config;
pub use rfs::config::file_content::FileContent;
pub use rfs::fs_tester::FsTester;
pub use rfs::fs_tester_error::{FsTesterError, Result};
