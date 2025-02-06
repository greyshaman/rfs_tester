use std::fs;

use rfs_test_macro::rfs_test;

const CONFIG: &str = r#"---
    - !directory
        name: test
        content:
        - !file
            name: file_link.txt
            content:
                !original_file Cargo.toml
"#;

#[rfs_test(config = CONFIG, start_point = ".")]
fn link_creation_test(dirname: &str) -> std::io::Result<()> {
    let file_path = format!("{dirname}/file_link.txt");
    let meta = fs::metadata(file_path)?;
    assert!(meta.is_file());
    Ok(())
}
