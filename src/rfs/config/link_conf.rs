use serde::{Deserialize, Serialize};

/// The structure of the configuration link
///
/// The link may refer to another test file.
///
/// ### yaml
///
/// ```yaml
/// - link:
///     name: test_link
///     target: test.txt
/// ```
///
/// ### json
/// ```json
/// "link": {
///   "name": "test_link",
///   "target": "test.txt"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct LinkConf {
    pub name: String,
    pub target: String,
}
