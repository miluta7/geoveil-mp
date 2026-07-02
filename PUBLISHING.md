# Release Guide

How to publish a new geoveil-mp version. Releases are fully automated by
GitHub Actions ([.github/workflows/ci.yml](.github/workflows/ci.yml)) —
pushing a `v*` tag builds wheels for Linux/Windows/macOS × Python 3.9–3.12
and publishes to PyPI via trusted publishing (no API token needed).

## Steps

1. **Bump versions** (keep them in sync):
   - `Cargo.toml` → `version = "X.Y.Z"`
   - `pyproject.toml` → `version = "X.Y.Z"`

2. **Update `CHANGELOG.md`** — add a `## [X.Y.Z] - YYYY-MM-DD` section
   (Keep a Changelog format: Added / Changed / Fixed).

3. **Test locally**:
   ```bash
   cargo test
   maturin build --release --features python
   pip install --force-reinstall target/wheels/geoveil_mp-*.whl
   python -c "import geoveil_mp as gm; print(gm.version())"
   ```

4. **Commit, tag, push**:
   ```bash
   git add -A
   git commit -m "vX.Y.Z: <summary>"
   git push origin main
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. **CI does the rest**: `test` → `test-python` → `build-wheels` (12-wheel
   matrix) → `publish` (PyPI, [pypi.org/project/geoveil-mp](https://pypi.org/project/geoveil-mp/)).
   Watch progress under the repo's Actions tab (~15 min).

6. **Create the GitHub Release**: Releases → Draft a new release → pick the
   tag, summarize the changelog, mark as latest.

## PyPI trusted publishing

The `publish` job authenticates with an OIDC token
(`permissions: id-token: write`, environment `pypi`) — configured once on
PyPI under *Manage project → Publishing*. If publishing fails with an
authentication error, re-check that the GitHub repository / workflow /
environment names in the PyPI trusted-publisher settings still match.

## crates.io (optional)

The Rust crate is not currently published to crates.io. If desired:
```bash
cargo publish --dry-run
cargo publish
```

## Versioning

Semantic versioning: breaking Python-API changes bump the minor version
while < 1.0 (0.1 → 0.2); patch releases for fixes. MP RMS values and
statistics key formats (e.g. `GPSM1C`) are observable behavior for
downstream consumers (GeoVeil batch) — call out any change to them in the
changelog.
