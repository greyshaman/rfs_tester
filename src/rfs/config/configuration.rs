use serde::{Deserialize, Serialize};

use super::config_entry::ConfigEntry;

/// File System config structure to contains directories, files and links
/// to execute tests with fs io operations
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Configuration(pub Vec<ConfigEntry>);
