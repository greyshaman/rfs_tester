use serde::{Deserialize, Serialize};

use super::{
    clone_directory_conf::CloneDirectoryConf, directory_conf::DirectoryConf, file_conf::FileConf,
    link_conf::LinkConf,
};

/// A configuration item can be a directory, file, or link.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigEntry {
    /// The Directory directive specifies the directory that will be created,
    /// with the name provided in the "name" parameter,
    /// and with the content specified in the "content" parameter.
    Directory(DirectoryConf),

    /// The Clone Directory directive is used to create a new folder with a name
    /// based on the "name" parameter and a copy of all files and folders
    /// from the specified source folder to folder with name from "source" parameter.
    CloneDirectory(CloneDirectoryConf),

    /// The File directive is used to create a file
    /// with a name specified in the "name" property
    /// and content configured by the "content" property value.
    File(FileConf),

    /// The Link directive is used to create a link to a file.
    /// This is useful if you need to access the contents of a large file
    /// without having to copy it. However, __YOU SHOULD BE CAREFUL__ when using this directive
    /// as it can potentially violate the integrity of the file's data.
    /// By default, working with files through links is disabled,
    /// and an error will occur if you try to do so.
    /// To enable this feature, you can set the LINKS_ALLOWED environment variable to true.
    Link(LinkConf),
}
