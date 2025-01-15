use serde::{Deserialize, Serialize};

use super::{directory_conf::DirectoryConf, file_conf::FileConf, link_conf::LinkConf};

/// FS Config entry enum
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigEntry {
    Directory(DirectoryConf),
    File(FileConf),
    Link(LinkConf),
}
