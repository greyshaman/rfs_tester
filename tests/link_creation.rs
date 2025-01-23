use std::fs;

use rfs_test_macro::rfs_test;

const CONFIG: &str = r#"---
    - !directory
        name: test
        content:
        - !link
            name: file_link.txt
            target: LICENSE-MIT
"#;

#[rfs_test(config = CONFIG, start_point = ".")]
fn link_creation_test(dirname: &str) -> std::io::Result<()> {
    let link_path = format!("{dirname}/file_link.txt");
    let meta = fs::metadata(link_path)?;
    assert!(meta.is_file());
    Ok(())
}
