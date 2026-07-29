# SBOM Release Artefact

Each tagged release publishes a `propchain-sbom.json` asset generated from the
workspace with `cargo sbom`.

## What ships

- **Format**: SPDX JSON 2.3
- **Asset name**: `propchain-sbom.json`
- **Source**: the Cargo workspace rooted at this repository
- **Release hook**: `.github/workflows/release.yml`

## What to look for

The most useful top-level fields are:

- `SPDXID`: confirms the document identifier.
- `creationInfo`: records when the SBOM was produced and which tool created it.
- `packages`: lists workspace crates and third-party dependencies captured from
  Cargo metadata.
- `relationships`: records the dependency graph between the packages in the
  document.

## Validation

The release workflow validates the generated artefact before upload by checking
that the document has the expected SPDX identifier plus non-empty
`creationInfo.creators`, `packages`, and `relationships` sections.

You can reproduce the same flow locally:

```bash
cargo install cargo-sbom --locked
cargo sbom --output-format spdxjson23 > propchain-sbom.json
jq -e '
  .SPDXID == "SPDXRef-DOCUMENT" and
  (.creationInfo.creators | length > 0) and
  (.packages | length > 0) and
  (.relationships | length > 0)
' propchain-sbom.json > /dev/null
```

## How to use it

- Review `packages` to see the exact dependency inventory captured for a
  release.
- Review `relationships` to understand how a crate is pulled into the workspace.
- Pair the SBOM with `cargo deny check` and advisory scanning for compliance and
  vulnerability triage.
