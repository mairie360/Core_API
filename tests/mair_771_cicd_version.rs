//! MAIR-771: the GDPR controls of mairie360/CICD (`gdpr_contract` on CI, `gdpr_marker` and
//! `gdpr_simulation` on promote-prod) only exist from CICD v4.12.0. This guard fails when a
//! workflow of this repo falls behind that release, or when the workflows disagree on the
//! version they call (the `uses:` ref and `cicd_version` must stay equal, since the actions are
//! checked out at `cicd_version`).

use std::fs;
use std::path::Path;

const MIN_CICD: (u64, u64, u64) = (4, 12, 0);
const WORKFLOWS: [&str; 4] = [
    "cicd.yml",
    "gdpr-pr.yml",
    "promote-staging.yml",
    "promote-prod.yml",
];

fn read_workflow(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/workflows")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.strip_prefix('v')?.split('.').map(str::parse::<u64>);
    let version = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    parts.next().is_none().then_some(version)
}

/// Every CICD version a workflow names: the `uses:` refs (the `# vX.Y.Z` comment of a SHA pin)
/// and the `cicd_version:` inputs.
fn cicd_versions(workflow: &str) -> Vec<String> {
    let mut versions = Vec::new();
    for line in workflow.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("uses: mairie360/CICD/") {
            let reference = rest.split_once('@').map_or("", |(_, r)| r);
            let (pin, comment) = reference.split_once('#').unwrap_or((reference, ""));
            let comment = comment.trim();
            versions.push(
                if comment.is_empty() {
                    pin.trim()
                } else {
                    comment
                }
                .to_owned(),
            );
        } else if let Some(rest) = line.strip_prefix("cicd_version:") {
            versions.push(rest.split('#').next().unwrap_or("").trim().to_owned());
        }
    }
    versions
}

#[test]
fn mair_771_every_workflow_calls_the_same_cicd_release_with_the_gdpr_controls() {
    let mut seen: Vec<(String, String)> = Vec::new();
    for name in WORKFLOWS {
        let versions = cicd_versions(&read_workflow(name));
        assert!(!versions.is_empty(), "{name} does not call mairie360/CICD");
        seen.extend(versions.into_iter().map(|v| (name.to_owned(), v)));
    }
    let first = seen[0].1.clone();
    for (name, version) in &seen {
        assert_eq!(
            version, &first,
            "{name} pins CICD {version}, expected {first} everywhere"
        );
    }
    let parsed = parse_version(&first).unwrap_or_else(|| panic!("CICD pin {first} is not vX.Y.Z"));
    assert!(
        parsed >= MIN_CICD,
        "CICD {first} predates v4.12.0: no gdpr_contract, gdpr_marker nor gdpr_simulation"
    );
}

#[test]
fn mair_771_ci_and_prod_promotion_hand_the_ai_review_its_key() {
    for name in ["cicd.yml", "promote-prod.yml"] {
        let workflow = read_workflow(name);
        assert!(
            workflow.contains("secrets: inherit") || workflow.contains("ANTHROPIC_API_KEY:"),
            "{name} does not pass ANTHROPIC_API_KEY: the AI part of the GDPR controls is skipped"
        );
    }
}

#[test]
fn mair_771_version_parser_rejects_what_is_not_a_release() {
    assert_eq!(parse_version("v5.2.2"), Some((5, 2, 2)));
    assert!(parse_version("v4.4.3").is_some_and(|v| v < MIN_CICD));
    assert_eq!(parse_version("main"), None);
    assert_eq!(parse_version("v5.2"), None);
    assert_eq!(
        cicd_versions("    uses: mairie360/CICD/.github/workflows/x.yml@0123abc # v5.2.2\n      cicd_version: v5.2.2  # note"),
        vec!["v5.2.2".to_owned(), "v5.2.2".to_owned()]
    );
}
