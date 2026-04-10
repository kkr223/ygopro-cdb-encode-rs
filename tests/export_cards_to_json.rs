use std::{fs, path::PathBuf};

#[test]
fn read_cdb_and_write_json_snapshot() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cdb_path = repo_root.join("cards.cdb");
    assert!(
        cdb_path.exists(),
        "expected sample cdb at {}",
        cdb_path.display()
    );

    let cdb = ygopro_cdb_encode_rs::YgoProCdb::from_path(&cdb_path).expect("open cdb");
    let cards = cdb.find_all().expect("read all cards");
    assert!(!cards.is_empty(), "expected at least one card");

    let output_dir = repo_root.join("target").join("test-artifacts");
    fs::create_dir_all(&output_dir).expect("create output dir");

    let output_path = output_dir.join("cards.json");
    let json = serde_json::to_string_pretty(&cards).expect("serialize cards");
    fs::write(&output_path, json).expect("write json");

    assert!(
        output_path.exists(),
        "expected output json at {}",
        output_path.display()
    );
}
