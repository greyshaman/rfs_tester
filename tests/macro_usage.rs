use rfs_test_macro::rfs_test;

#[rfs_test(
    config = r#"---
    - !directory
        name: test
        content:
          - !file
              name: test.txt
              content:
                !inline_text "Hello, world!"
    "#,
    start_point = "."
)]
fn file_creation_test_with_macro(dirname: &str) -> std::io::Result<()> {
    let file_path = std::path::PathBuf::from(dirname).join("test.txt");
    let content = std::fs::read_to_string(file_path)?;
    assert_eq!(content, "Hello, world!");
    Ok(())
}

#[rfs_test(
    config = r#"---
    - !directory
        name: test
        content:
          - !file
              name: test1.txt
              content:
                !inline_text "File 1"
          - !file
              name: test2.txt
              content:
                !inline_text "File 2"
    "#,
    start_point = "."
)]
fn multiple_files_test(dirname: &str) -> std::io::Result<()> {
    let current_dir = std::path::PathBuf::from(dirname);
    let file1_path = current_dir.join("test1.txt");
    let file2_path = current_dir.join("test2.txt");
    let content1 = std::fs::read_to_string(file1_path)?;
    let content2 = std::fs::read_to_string(file2_path)?;
    assert_eq!(content1, "File 1");
    assert_eq!(content2, "File 2");
    Ok(())
}
