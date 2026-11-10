#![cfg(feature = "editor")]

use breevy_core::gui::BreevyApp;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

fn sample_copy(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("breevy-kittest-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("abbreviations.bvy");
    std::fs::write(&path, include_str!("../sample.bvy")).unwrap();
    path
}

#[test]
fn renders_editor_with_all_controls() {
    let path = sample_copy("render");
    let mut app = BreevyApp::from_path(path.to_string_lossy().into());
    let harness = Harness::new(move |ctx| app.ui(ctx));
    harness.get_by_label("New Abbreviation");
    harness.get_by_label("Apply");
}

#[test]
fn add_snippet_then_apply_writes_file() {
    let path = sample_copy("apply");
    let mut app = BreevyApp::from_path(path.to_string_lossy().into());
    let mut harness = Harness::new(move |ctx| app.ui(ctx));
    harness.run();

    harness.get_by_label("New Abbreviation").click();
    harness.run();
    harness.get_by_label("Apply").click();
    harness.run();

    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("new9"), "saved file should contain the new abbreviation");
    assert_eq!(saved.matches("<a a=").count(), 9);
}
