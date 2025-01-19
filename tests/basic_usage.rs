use rfs_tester::config::{ConfigEntry, Configuration, DirectoryConf, FileConf};
use rfs_tester::{FileContent, FsTester};

#[test]
fn basic_test_file_creation() {
    let config_str = r#"---
    - !directory
        name: test
        content:
          - !file
              name: test.txt
              content:
                !inline_text "Hello, world!"
    "#;

    // Creates a temporary file system
    let tester = FsTester::new(config_str, ".");

    // Performs the test
    tester.perform_fs_test(|dirname| {
        let file_path = format!("{}/test.txt", dirname);
        let content = std::fs::read_to_string(file_path)?;
        assert_eq!(content, "Hello, world!");
        Ok(())
    });
}
