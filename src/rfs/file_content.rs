use serde::{Deserialize, Serialize};

/// File content can be presented in fourth ways:
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileContent {
    /// InlineBytes - by byte vector:
    ///
    /// ```yaml
    /// - !directory
    ///     name: base_container
    ///     content:
    ///         - !file
    ///             name: test.txt
    ///             content:
    ///                 !inline_bytes
    ///                     - 116
    ///                     - 101
    ///                     - 115
    ///                     - 116
    /// ```
    InlineBytes(Vec<u8>),
    /// InlineText - by usual string of text:
    ///
    /// ```yaml
    /// - !directory
    ///     name: base_container
    ///     content:
    ///         - !file
    ///             name: test.txt
    ///             content:
    ///                 !inline_text test
    /// ```
    InlineText(String),
    /// OriginalFile - Retrieve from a real file using its path:
    ///
    /// ```yaml
    /// - !directory
    ///     name: base_container
    ///     content:
    ///         - file
    ///             name: test.txt
    ///             content:
    ///                 !original_file "test.txt"
    /// ```
    OriginalFile(String),
    /// or simply Empty
    ///
    /// ```yaml
    /// - !directory
    ///     name: base_container
    ///     content:
    ///         - !file
    ///             name: test.txt
    ///             content: !empty
    Empty,
}
