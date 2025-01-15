use serde::{Deserialize, Serialize};

use crate::rfs::file_content::FileContent;

/// The structure for file records in the configuration.
/// The file can be configured in three ways: as an empty file,
/// using a bytes array, or by referencing a real file whose contents
/// will be used for the test file.
///
/// ## Empty file
///
/// ### yaml:
///
/// ```yaml
/// - file:
///     name: test.txt
///     content: empty
/// ```
///
/// ### json:
///
/// ```json
/// "file": {
///   "name": "test.txt",
///   "content": "empty"
/// }
/// ```
/// ## InlineBytes
/// File content can be configured using **inline_bytes**. If you choose to use inline,
/// you will need to add a bytes array to the configuration.
/// This configuration is only useful for small test files.
///
/// Example of configuration for a file named **test.txt** with the content "test":
///
/// ### yaml
///
/// ```yaml
/// - file:
///     name: test.txt
///     content:
///       inline_bytes:
///         - 106
///         - 101
///         - 115
///         - 116
/// ```
///
/// ### json
///
/// ```json
/// "file": {
///   "name": "test.txt",
///   "content": {
///     "inline_bytes": [116, 101, 115, 116]
///   }
/// }
/// ```
///
/// ## Original file
/// In case we need a larger file, we can use **inline_bytes** or **inline_text** to specify it.
/// We can also specify the path to the original file on the file system
/// and have its contents copied to the test file.
///
/// For example, we have a music.mp3 file
/// and we want to create a test file that has the same content.
///
/// ### yaml
///
/// ```yaml
/// - file:
///     name: test.mp3
///     content:
///       original_file: "./music.mp3"
/// ```
///
/// ### json
/// ```json
/// "file" : {
///   "name": "test.mp3",
///   "content": {
///     "original_file": "./music.mp3"
///   }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileConf {
    pub name: String,
    pub content: FileContent,
}
