# Release provenance

[Back to README](../README.md) · [Setup](setup.md)

## Scope

The release workflow now generates signed GitHub build-provenance attestations
for each architecture's final stripped executable, archive, checksum manifest,
and build-input record. The archive covers the bundled udev rule and documentation
as well as the executable. Each architecture has its own Sigstore bundle.

This applies only to releases built with the updated workflow. The existing
v0.6.1 release has no established source-to-binary provenance. Its matching AUR
checksums identify the downloaded archives; they do not establish their source.
Do not sign downloaded old assets and describe that as their original build
provenance, move the v0.6.1 tag, or replace its assets.

Provenance authenticates a workflow's claim connecting artifact digests to source
and build identity. It does not independently establish that a build is safe or
reproducible. Reproducible-build equivalence requires an independent rebuild and
matching bytes. Rust and cross versions and action revisions are pinned, but the
Ubuntu 24.04 runner image, system packages, and cross image are still mutable. Build records help
investigation; they are not a complete reproducible build environment. Archive
timestamps, ownership, and ordering would also need normalization.

## Verify a release before installation

Use a current GitHub CLI supporting `gh attestation verify`, Bash, and tar.
Select a release built with this workflow and obtain its full, reviewed source
commit independently of the downloaded build-info file. In a clean directory,
set these values (the placeholders must be replaced):

```bash
tag=vMAJOR.MINOR.PATCH
commit=FULL_REVIEWED_40_CHARACTER_SOURCE_COMMIT
arch=x86_64 # or aarch64
base="nuphyctl-$tag-linux-$arch"
gh release download "$tag" --repo fldc/nuphyctl --pattern "$base.*"
```

Use the verification script from a trusted checkout of this repository:

```bash
/path/to/trusted/nuphyctl/scripts/verify-release.sh "$tag" "$commit" "$arch" .
```

The script verifies the downloaded archive before extracting the executable into
a temporary directory. It then verifies that executable, the checksums, and the
build-input record. It never runs the downloaded executable. Any verification
failure stops the script with a nonzero status. The two verification JSON files
are local results; downloaded copies of those files are not a substitute for
running verification yourself.

Verification requires the expected repository, release workflow, source tag,
source commit, and GitHub-hosted runner. The script uses the shipped bundle,
whose signature must validate even though the bundle comes from the same download
location. For an additional lookup against the current GitHub repository, run:

```bash
gh attestation verify "$base.tar.gz" \
  --repo fldc/nuphyctl \
  --signer-workflow fldc/nuphyctl/.github/workflows/release.yml \
  --source-ref "refs/tags/$tag" --source-digest "$commit" \
  --deny-self-hosted-runners
```

Bundle verification checks repository identity as recorded at signing time;
online lookup also associates the attestation with the current repository.
Bundle verification can still need network access for trust material; the script
is not an air-gapped verification setup. See the
[GitHub CLI verification documentation](https://cli.github.com/manual/gh_attestation_verify).

After verification succeeds, extract the archive and follow the
[installation instructions](setup.md#release-binaries).

## Maintainer release procedure

1. Review source and workflow changes, set the new Cargo package version, and
   commit them. Create a new version tag at that commit; do not reuse an old tag.
2. Push the new tag to run the release workflow. Manual dispatch must also select
   a version tag matching Cargo.toml. Both architecture jobs build, strip, package,
   record inputs, attest, and preserve the signed bundles.
3. The release job verifies downloaded artifacts for both architectures against
   the tag's source commit before creating a draft release and publishing it.
   It refuses to reuse an existing release. If upload or publication fails after
   draft creation, inspect that draft and its artifacts before recovery; a rerun
   will not overwrite it.
4. Independently download and verify both published architecture sets. Preserve
   the verification output, full source commit, artifact SHA-256 values,
   attestation URLs, and workflow run/attempt URL. The build-info file records the
   run URL; the build job summary links its attestation.

No local check can exercise GitHub's OIDC signing. Successful signed release
generation and independent verification are required before claiming provenance
for a particular published artifact.

## AUR packaging

The AUR packaging is maintained outside this repository. For its next update:

- Verify both architectures' downloaded archives and extracted executables before
  accepting new checksums. Use version-specific release URLs in `source_x86_64`
  and `source_aarch64`, with matching `sha256sums_x86_64` and
  `sha256sums_aarch64`. Do not use `/latest/` or `SKIP` for archives.
- Take archive hashes from the verified `.SHA256SUMS` files. Keep the reviewed
  source commit, verification results, attestation links, and release-run URL with
  the packaging change. Regenerate `.SRCINFO`. Standard makepkg checksum checks
  do not verify GitHub provenance; this is a separate packaging acceptance step.
- Preserve the already stripped executable bytes, using `options=('!strip')`,
  and compare the packaged executable's SHA-256 with the attested executable.
  Update `pkgver` for a new upstream release and reset `pkgrel` to 1.

Evidence addressing the marketplace prerequisite consists of successful fresh
verification for both architectures' archives and executable digests, tied to a
reviewed full source commit and workflow run, plus an AUR revision consuming those
exact assets. That addresses the provenance concern; marketplace maintainer
approval remains a separate decision. A new release does not establish provenance
for the old nuphyctl-bin 0.6.1-1 dependency.

## Third-party attribution

Future binary archives include `THIRD-PARTY-NOTICES/`. Its `components.json`
records the default-feature Cargo graph for the target, versions, features,
registry archive checksums, notice paths, executable digest, source commit, and
Rust version. Original texts are taken from checksum-verified registry archives;
HIDAPI's bundled C notices and Rust's standard-library copyright mapping and
license texts are included. Build/proc-macro helpers are conservatively included
and labeled separately. The graph does not assert which functions survive linking.
System shared libraries remain supplied by the operating system.

For existing v0.6.2 binaries, the manually dispatched `release-notices.yml`
workflow publishes architecture-specific `*-notices.tar.gz` supplements with a
separate attestation. It verifies the original archives/executables, checks out
the original source commit, and generates notices with Rust 1.98.0 without
executing or rebuilding the binaries. Existing signed archives stay intact.
The supplement's attestation identifies the notice-generation workflow and its
source commit; it is not a new attestation of the original binary build.

Download the notice supplement alongside the matching v0.6.2 binary archive.
AUR packaging must download both and install the notice tree under
`/usr/share/licenses/nuphyctl-bin/THIRD-PARTY-NOTICES/`, preserving nested files
and Rust's HTML/license links. Newer releases carry the same tree in the main
archive. Check the supplement against its own attestation using
`gh attestation verify` with signer workflow
`fldc/nuphyctl/.github/workflows/release-notices.yml` and the expected generation
commit, rather than the binary's source commit.
