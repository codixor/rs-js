use std::fs;
use std::path::PathBuf;

pub fn assert_golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name);

    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        fs::create_dir_all(path.parent().expect("golden path should have a parent"))
            .expect("golden dir should be creatable");
        fs::write(&path, actual).expect("golden file should be writable");
    }

    let expected =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {}", path.display()));
    assert_eq!(
        actual,
        expected,
        "golden output mismatch for {}",
        path.display()
    );
}
