# GeoVeil-MP Publishing Guide

## 📦 Package Contents

The `geoveil-mp.tar.gz` contains a complete Rust+Python library ready for publishing.

## 🚀 Publishing to GitHub

### 1. Create GitHub Repository

```bash
# Go to GitHub and create a new repository named "geoveil-mp"
# URL: github.com/miluta7/geoveil-mp
```

### 2. Initialize Git and Push

```bash
# Extract the package
tar -xzf geoveil-mp.tar.gz
cd geoveil-mp

# Initialize git
git init
git add .
git commit -m "Initial release of GeoVeil-MP v0.1.0"

# Add remote and push
git remote add origin https://github.com/miluta7/geoveil-mp.git
git branch -M main
git push -u origin main
```

### 3. Create Release Tag

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

This will trigger the GitHub Actions workflow to build and publish wheels.

## 📤 Publishing to PyPI

### Option A: Automatic (via GitHub Actions)

1. Set up PyPI secrets in GitHub:
   - Go to Settings > Secrets > Actions
   - Add `PYPI_API_TOKEN` with your PyPI API token
   - Add `CRATES_IO_TOKEN` (optional, for crates.io)

2. Push a version tag:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

3. GitHub Actions will automatically build and publish wheels.

### Option B: Manual Publishing

```bash
# Install dependencies
pip install maturin twine

# Build wheels
maturin build --release --features python

# Upload to PyPI
twine upload target/wheels/*.whl

# Or upload to TestPyPI first
twine upload --repository testpypi target/wheels/*.whl
```

## 🦀 Publishing to crates.io

```bash
# Login to crates.io
cargo login

# Publish (dry-run first)
cargo publish --dry-run

# Publish for real
cargo publish
```

## ✅ Post-Publishing Verification

### Test PyPI Installation

```bash
pip install geoveil-mp
python -c "import geoveil_mp as gm; print(gm.version())"
```

### Test Cargo Installation

```bash
cargo install geoveil_mp
geoveil-mp --version
```

## 📋 Checklist

- [ ] GitHub repository created at github.com/miluta7/geoveil-mp
- [ ] Code pushed to GitHub
- [ ] `PYPI_API_TOKEN` secret configured
- [ ] `CRATES_IO_TOKEN` secret configured (optional)
- [ ] Version tag pushed
- [ ] GitHub Actions workflow completed
- [ ] PyPI package verified
- [ ] crates.io package verified (optional)

## 🔗 Links

After publishing, your package will be available at:

- **GitHub**: https://github.com/miluta7/geoveil-mp
- **PyPI**: https://pypi.org/project/geoveil-mp/
- **crates.io**: https://crates.io/crates/geoveil-mp (if published)
- **docs.rs**: https://docs.rs/geoveil-mp (automatic after crates.io)

## 📝 Version Updates

To release a new version:

1. Update version in `Cargo.toml` and `pyproject.toml`
2. Update `CHANGELOG.md`
3. Commit and push changes
4. Create and push new tag: `git tag vX.Y.Z && git push origin vX.Y.Z`
