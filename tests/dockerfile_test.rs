//! Regression guard for MAIR-741: the builder, runtime and development images must share one
//! Debian release.
//!
//! The release binary is linked in the builder stage against its glibc and OpenSSL, then run on
//! the distroless runtime image: a newer builder than runtime produces a binary that does not
//! start, and an older runtime brings back the unpatched `libssl3` of Debian 12 (CVE-2026-84782).

use std::path::Path;

/// Returns the image references of every `FROM` instruction of a Dockerfile of the repo.
fn from_images(file: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    content
        .lines()
        .map(str::trim)
        .filter(|line| line.to_ascii_uppercase().starts_with("FROM "))
        .filter_map(|line| line.split_whitespace().nth(1).map(str::to_owned))
        .collect()
}

/// Debian major version an image is built on, read from its tag (`trixie`, `debian13`…).
fn debian_release(image: &str) -> u32 {
    let releases = [
        ("bookworm", 12),
        ("debian12", 12),
        ("trixie", 13),
        ("debian13", 13),
    ];
    releases
        .iter()
        .find(|(marker, _)| image.contains(marker))
        .map_or_else(
            || panic!("cannot tell the Debian release of `{image}`"),
            |(_, release)| *release,
        )
}

#[test]
fn mair_741_every_image_runs_on_debian_13() {
    for file in ["Dockerfile", "development.Dockerfile"] {
        for image in from_images(file) {
            assert_eq!(
                debian_release(&image),
                13,
                "{file}: `{image}` must be on Debian 13, like the distroless runtime image"
            );
        }
    }
}
