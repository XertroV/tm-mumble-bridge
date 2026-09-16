# Releasing tm-mumble-link

A release is finished only when its GitHub Release is published with its complete
changelog and both platform archives and checksums attached and verified.
Creating a tag or starting CI is not completion. Agents should own this workflow
through CI failures, review feedback, publishing, and final verification.

## Prepare and review

1. Inspect the latest published GitHub Release and its tag, then inspect **every
   commit and merged PR since that tag** (`git log <prior-tag>..HEAD`). Include
   fixes, features, behavior changes, build/distribution changes, and limitations.
   For the first release, inspect all history and say it is the first release.
   Do not rely only on GitHub's generated PR list: it can omit direct commits.
2. Choose an unused stable `X.Y.Z` version. Update `[package].version` in
   `Cargo.toml` and the `tm-mumble-link` entry in `Cargo.lock` together. Other
   dependency versions should not change just to release.
3. Write `releases/vX.Y.Z.md`, starting with `# vX.Y.Z`, with the complete curated
   changelog. Include the previous release/compare link when one exists. This
   exact file becomes the body of the GitHub Release; CI does not invent notes.
4. Commit and push the version and notes on a PR branch. Check that **Build and
   test** actually starts, inspect results, and fix failures. Wait for CodeRabbit,
   address its concerns, and wait for the final revision's checks before merging.
   Ordinary PR/branch CI creates release-mode Linux and Windows archives as
   downloadable Actions artifacts without publishing a GitHub Release.

## Start the release

There are two entry points using the same Release workflow:

- **Prepared release PR (agent-friendly):** merging `releases/vX.Y.Z.md` to
  `master` starts **Prepare release tag**. It validates both Cargo versions and
  notes, creates `vX.Y.Z` at that exact merge commit, then dispatches **Release**
  on the tag. Merging release notes is an explicit request to publish that version.
  Do not add draft release-note files to `master` ahead of the release.
- **Existing prepared commit:** push a new version tag on a commit already merged
  into `master`: `git tag vX.Y.Z <commit>` then `git push origin vX.Y.Z`. New `v*`
  tags start **Release** automatically. The tagged commit must contain matching
  Cargo versions and `releases/vX.Y.Z.md`.

GitHub does not trigger push workflows for tags created using `GITHUB_TOKEN`,
so the preparation workflow explicitly dispatches Release after creating its tag.
This uses the repository's built-in token, with no PAT or extra secret required.
Repository Actions policy must allow the referenced actions; preparation needs
`contents: write` and `actions: write`, publishing needs `contents: write`.
Build/test jobs have read-only repository access, including on fork PRs.

## Build and publication gates

**Release** rejects a branch dispatch, a mismatched version, missing/empty notes,
or a tag whose commit is not an ancestor of `master`. It then calls the same
reusable build workflow as ordinary CI:

- Linux x86_64 GNU on Ubuntu 24.04: `cargo test --locked`, optimized release build,
  and the GUI/TCP/Mumble shared-memory smoke test against the **release binary**.
- Windows x86_64 MSVC: `cargo test --locked` and optimized release build.
- Each job packages its binary with `README.md`, `LICENSE`, and this document,
  creates a SHA-256 sidecar, and uploads Actions artifacts (retained for 14 days).

After both succeed, the publisher downloads exactly four files:

- `tm-mumble-link-vX.Y.Z-linux-x86_64.tar.gz`
- `tm-mumble-link-vX.Y.Z-linux-x86_64.tar.gz.sha256`
- `tm-mumble-link-vX.Y.Z-windows-x86_64.zip`
- `tm-mumble-link-vX.Y.Z-windows-x86_64.zip.sha256`

It validates their hashes, creates/updates a **draft** GitHub Release using the
committed notes, uploads all assets, downloads them again to verify their bytes,
and only then publishes. It checks the published release and assets once more.
The Linux tarball preserves the binary's executable bit. The archives are not
static distributions: Linux requires glibc 2.39+ (Ubuntu 24.04 or compatible),
desktop/graphics libraries described in README, and a working graphics driver.
Windows builds are unsigned. Neither platform packages Mumble or Trackmania.

## Monitor, repair, verify

- Watch **Prepare release tag** (if used) and **Release** until they finish.
  Inspect failing job logs, fix the root cause, commit changes, and rerun CI.
- A transient runner/upload failure can be retried with **Re-run failed jobs**.
  To rerun the complete release: `gh workflow run release.yml --ref vX.Y.Z`.
  Always dispatch on the tag, not `master`. A failed preparation dispatch can be
  retried; it accepts an existing tag only if it points to the exact same commit.
- If code, workflow, dependencies, or notes need to change **after tagging**, make
  a new patch version and changelog PR. Never move/delete an existing release tag
  to point at different source. An incomplete draft can remain until explicitly
  cleaned up; it is not a successful release.
- Upload retries replace files only on a draft. An already-published release is
  verified without altering its files or notes. Rebuilt archives can have different
  timestamps, so verification uses the published checksums on a completed rerun.
- Verify the Release URL is public, its body matches the committed changelog, and
  all four correctly versioned assets are attached and nonempty. Download both
  archives and compare SHA-256 checksums. Report the release URL and CI result.

A successful automated run verifies packaging and the Linux shared-memory
integration. It does not verify audible in-game output with real players,
Proton integration, Windows GUI behavior, or Wayland desktop behavior.
