//! MAIR-243: the cargo tests (testcontainers) and the docker compose stacks must run against the
//! same Database release. `TEST_DB_VERSION` in `.cargo/config.toml` is the single declaration; every
//! `ghcr.io/mairie360/database` and `liquibase-migrations` image of the compose files must carry
//! that tag, which must be a released `X.Y.Z` (not a dev build that drifts per repo). The template
//! image name of the scaffold (`service_name`) must not come back either.

use std::fs;
use std::path::Path;

const MIN_DB: (u64, u64, u64) = (3, 4, 0);

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn test_db_version() -> String {
    read(".cargo/config.toml")
        .lines()
        .find_map(|line| line.trim().strip_prefix("TEST_DB_VERSION"))
        .and_then(|rest| rest.split('"').nth(1))
        .expect("TEST_DB_VERSION in .cargo/config.toml")
        .to_string()
}

fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.split('.').map(str::parse::<u64>);
    let version = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    parts.next().is_none().then_some(version)
}

fn compose_files() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("docker-compose") && name.ends_with(".yml"))
        .collect();
    names.sort();
    names
}

/// Tags of the Database images of one compose file.
fn database_tags(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let rest = line.split("ghcr.io/mairie360/").nth(1)?;
            let (image, tag) = rest.split_once(':')?;
            matches!(image, "database" | "liquibase-migrations")
                .then(|| tag.trim_end_matches(['}', '"', ' ']).to_string())
        })
        .collect()
}

#[test]
fn test_db_version_is_a_release_at_least_the_aligned_one() {
    let version = test_db_version();
    let parsed = parse_version(&version)
        .unwrap_or_else(|| panic!("TEST_DB_VERSION `{version}` must be a release X.Y.Z"));
    assert!(
        parsed >= MIN_DB,
        "TEST_DB_VERSION `{version}` is older than {MIN_DB:?}"
    );
}

#[test]
fn compose_files_use_the_test_db_version() {
    let version = test_db_version();
    for name in compose_files() {
        let tags = database_tags(&read(&name));
        assert!(!tags.is_empty(), "{name} pins no Database image");
        for tag in tags {
            assert_eq!(
                tag, version,
                "{name} runs another Database version than TEST_DB_VERSION"
            );
        }
    }
}

#[test]
fn compose_files_no_longer_name_the_template_image() {
    for name in compose_files() {
        assert!(
            !read(&name).contains("service_name"),
            "{name} still names the template image `service_name`"
        );
    }
}
