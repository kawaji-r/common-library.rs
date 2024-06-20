use std::fs;
use std::process::Command;

fn get_version_from_cargo_toml() -> Option<String> {
    let cargo_toml_content = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let cargo_toml: toml::Value = cargo_toml_content
        .parse()
        .expect("Failed to parse Cargo.toml");

    cargo_toml
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|version| version.as_str())
        .map(|s| s.to_string())
}

fn main() {
    // (1) Cargo.tomlのpackageセクションのversionの値を取得する。
    let version = get_version_from_cargo_toml().expect("Failed to get version from Cargo.toml");

    // (2) `git tag`を実行してtag名の一覧を取得する。
    let output = Command::new("git")
        .arg("tag")
        .output()
        .expect("Failed to execute git tag");

    let tags = String::from_utf8_lossy(&output.stdout);

    // (3) 取得したタグ一覧の中にバージョンがあるか確認。
    if !tags.contains(&version) {
        // (4) タグが存在しない場合、現在のコミットにタグを追加してpush。
        Command::new("git")
            .arg("tag")
            .arg(&version)
            .output()
            .expect("Failed to create git tag");
        Command::new("git")
            .arg("push")
            .arg("origin")
            .arg(&version)
            .output()
            .expect("Failed to push git tag");
        println!("タグを追加しました: {}", &version);
    }
}
