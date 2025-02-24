use serde::{Deserialize, Serialize};

/// Structure for directory record in configuration
/// for example:
///
/// ## yaml:
///
/// ```yaml
/// ---
///   - !clone_directory
///       name: test_dir
///       source: data_dir
/// ```
///
/// ## json:
///
/// ```json
/// {
///     [
///         "clone_directory": {
///             "name": "test_dir",
///             "source": "data_dir"
///         }
///     ]
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CloneDirectoryConf {
    /// A directory will be created with the given name.
    pub name: String,

    /// The name of the destination directory for the copy.
    pub source: String,
}
