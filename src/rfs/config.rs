//! The 'config` module is responsible for configuring the temporary file system.
//! Enables settings for directories, files, and links.

pub mod config_entry;
pub mod configuration;
pub mod directory_conf;
pub mod file_conf;
pub mod link_conf;

pub use config_entry::ConfigEntry;
pub use configuration::Configuration;
pub use directory_conf::DirectoryConf;
pub use file_conf::FileConf;
pub use link_conf::LinkConf;
