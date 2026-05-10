#![allow(dead_code)]

use std::fs;
use std::path::Path;

use operad::{
    ColorRgba, CpuSnapshotImage, CpuSnapshotRenderer, UiDocument, UiSize,
    DEFAULT_CPU_SNAPSHOT_BACKGROUND,
};

pub const SNAPSHOT_BACKGROUND: ColorRgba = DEFAULT_CPU_SNAPSHOT_BACKGROUND;
pub type RasterImage = CpuSnapshotImage;

pub fn render_document(document: &mut UiDocument, viewport: UiSize) -> RasterImage {
    CpuSnapshotRenderer::default()
        .render_document(document, viewport)
        .unwrap_or_else(|failure| panic!("{failure}"))
}

pub fn assert_snapshot(name: &str, image: &RasterImage, expected_hash: u64) {
    if let Ok(dir) = std::env::var("OPERAD_WRITE_SNAPSHOTS") {
        fs::create_dir_all(&dir).expect("create snapshot directory");
        image
            .write_ppm(Path::new(&dir).join(format!("{name}.ppm")))
            .unwrap_or_else(|failure| panic!("{failure}"));
    }
    let renderer = CpuSnapshotRenderer::default();
    let assertions = renderer
        .snapshot_assertions(name, image)
        .unwrap_or_else(|failure| panic!("{failure}"));
    assertions
        .require_min_changed_pixels_from(
            SNAPSHOT_BACKGROUND,
            image.width() * image.height() / 40 + 1,
        )
        .unwrap_or_else(|failure| panic!("{failure}"));
    assertions
        .require_hash(expected_hash)
        .unwrap_or_else(|failure| panic!("{failure}"));
}
