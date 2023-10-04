use std::{fs::File, io::Read, path::Path};

pub fn compare_files(actual_path: &Path, expected_path: &Path) -> (String, String) {
    let mut actual_file = File::open(&actual_path).unwrap();
    let mut actual = String::new();
    actual_file.read_to_string(&mut actual).unwrap();

    let mut expected_file = File::open(&expected_path).unwrap();
    let mut expected = String::new();
    expected_file.read_to_string(&mut expected).unwrap();

    (actual, expected)
}
