use serde::{Deserialize, Serialize};

use super::config_entry::ConfigEntry;

/// Structure for directory record in configuration
/// for example:
///
/// ## yaml:
///
/// ```yaml
/// ---
///   - !directory
///       name: test
///       content:
///         - !file
///             name: test.txt
///             content: empty
///         - !link
///             name: test_link
///             target: test.txt
/// ```
///
/// ## json:
///
/// ```json
/// {
///     [
///         "directory": {
///             "name": "test_dir",
///             "content": [
///                 "file": {
///                     "name": "test.txt",
///                     "content": "empty"
///                 },
///                 "link": {
///                     "name": "test_link",
///                     "target": "test.txt"
///                 }
///             ]
///         }
///     ]
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DirectoryConf {
    /// A directory will be created with the given name.
    pub name: String,

    /// The directory content can contain a list of various entries.
    pub content: Vec<ConfigEntry>,
}
