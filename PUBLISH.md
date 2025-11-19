# 🚀 Publishing Tempo CLI

This guide walks you through publishing Tempo to multiple package managers so users can install with `uv install tempo-cli`, `cargo install tempo-cli`, or `brew install tempo`.

## 📋 Prerequisites

Before publishing, ensure you have:

- [ ] **GitHub Repository** set up with your code
- [ ] **Rust/Cargo** installed and configured  
- [ ] **Python/pip** installed for PyPI publishing
- [ ] **API Keys** for package managers (created below)
- [ ] **Version Numbers** updated in all files

## 🐍 PyPI Publishing (for `uv install tempo-cli`)

### 1. Setup PyPI Account

```bash
# Create account at https://pypi.org/account/register/
# Create API token at https://pypi.org/manage/account/token/
```

### 2. Install Publishing Tools

```bash
pip install --upgrade build twine
```

### 3. Update Package Information

Edit these files with your details:

**`python-package/python-pkg/setup.py`:**
```python
author="Own Path"
author_email="brandy.daryl@gmail.com"  
url="https://github.com/own-path/vibe"
```

**`python-package/python-pkg/pyproject.toml`:**
```toml
authors = [
    {name = "Own Path", email = "brandy.daryl@gmail.com"},
]
```

### 4. Build and Publish

```bash
# Navigate to Python package directory
cd python-package/python-pkg/

# Build the package
python -m build

# Check the package
twine check dist/*

# Upload to PyPI (test first)
twine upload --repository testpypi dist/*

# If test successful, upload to PyPI
twine upload dist/*
```

### 5. Test Installation

```bash
# Test the published package
pip install tempo-cli

# Verify it works
tempo --help
```

## 📦 Cargo/Crates.io Publishing (for `cargo install tempo-cli`)

### 1. Setup Crates.io Account

```bash
# Create account at https://crates.io/
# Get API token from https://crates.io/me
cargo login <your-api-token>
```

### 2. Update Cargo.toml

**`Cargo.toml`:**
```toml
[package]
name = "tempo-cli"
version = "1.0.0"
edition = "2021"
authors = ["Own Path <brandy.daryl@gmail.com>"]
license = "MIT"
description = "The Most Advanced Automatic Project Time Tracker"
repository = "https://github.com/own-path/vibe"
homepage = "https://github.com/own-path/vibe"
documentation = "https://docs.rs/tempo-cli"
keywords = ["time-tracking", "productivity", "cli", "terminal", "rust"]
categories = ["command-line-utilities", "development-tools"]
readme = "README.md"
```

### 3. Publish to Crates.io

```bash
# Check for issues
cargo check

# Build and test
cargo build --release
cargo test

# Publish (dry run first)
cargo publish --dry-run

# Actual publish
cargo publish
```

### 4. Test Installation

```bash
# Test the published crate
cargo install tempo-cli

# Verify it works
tempo --help
```

## 🍺 Homebrew Publishing (for `brew install tempo`)

### 1. Update Homebrew Formula

**`Formula/tempo.rb`:**
```ruby
class Tempo < Formula
  desc "The Most Advanced Automatic Project Time Tracker"
  homepage "https://github.com/own-path/vibe"
  url "https://github.com/own-path/vibe/archive/v1.0.0.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    system "#{bin}/tempo", "--version"
  end
end
```

### 2. Create Homebrew Tap

```bash
# Create a new GitHub repository named 'homebrew-tempo'
# Add the formula file to it

git clone https://github.com/own-path/homebrew-tempo.git
cd homebrew-tempo
cp ../tempo/Formula/tempo.rb ./Formula/tempo.rb
git add .
git commit -m "Add tempo formula"
git push origin main
```

### 3. Test Formula

```bash
# Test locally
brew install --build-from-source ./Formula/tempo.rb

# Test from tap
brew tap yourusername/tempo
brew install tempo
```

### 4. Submit to Homebrew Core (Optional)

For inclusion in Homebrew's main repository:
```bash
# Fork homebrew-core
# Add your formula to Formula/
# Submit pull request
```

## 📱 Additional Distribution Channels

### Arch User Repository (AUR)

Create `PKGBUILD`:
```bash
pkgname=tempo-cli
pkgver=1.0.0
pkgrel=1
pkgdesc="The Most Advanced Automatic Project Time Tracker"
arch=('x86_64')
url="https://github.com/own-path/vibe"
license=('MIT')
depends=()
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/own-path/vibe/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/tempo-$pkgver"
    cargo build --release
}

package() {
    cd "$srcdir/tempo-$pkgver"
    install -Dm755 target/release/tempo "$pkgdir/usr/bin/tempo"
}
```

### Windows Package Manager (winget)

Create manifest for winget-pkgs repository.

### Nix Package Manager

Add to nixpkgs or create flake.nix.

## ✅ Publishing Checklist

### Before Publishing

- [ ] Update version numbers in all files
- [ ] Update documentation and README
- [ ] Test build on all platforms
- [ ] Run comprehensive tests
- [ ] Update CHANGELOG.md
- [ ] Tag release in Git

### PyPI Publishing

- [ ] Update python package metadata
- [ ] Build and test package locally
- [ ] Upload to test.pypi.org first
- [ ] Test installation from test PyPI
- [ ] Upload to production PyPI
- [ ] Test installation: `uv install tempo-cli`

### Crates.io Publishing

- [ ] Update Cargo.toml metadata
- [ ] Run `cargo publish --dry-run`
- [ ] Publish to crates.io
- [ ] Test installation: `cargo install tempo-cli`

### Homebrew Publishing

- [ ] Update formula with correct SHA256
- [ ] Test formula locally
- [ ] Push to homebrew tap
- [ ] Test installation: `brew install yourusername/tempo/tempo`

### Post-Publishing

- [ ] Update documentation with install instructions
- [ ] Announce on social media / forums
- [ ] Monitor for issues and feedback
- [ ] Update package indexes if needed

## 🛠️ Automation Scripts

### Publish Script

Create `scripts/publish.sh`:
```bash
#!/bin/bash
set -e

VERSION=${1:-$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')}

echo "Publishing Tempo v$VERSION..."

# Update versions
sed -i "s/version = \".*\"/version = \"$VERSION\"/" Cargo.toml
sed -i "s/version=\".*\"/version=\"$VERSION\"/" python-package/python-pkg/setup.py

# Build and test
cargo build --release
cargo test

# Publish to crates.io
cargo publish

# Publish to PyPI
cd python-package/python-pkg/
python -m build
twine upload dist/*

echo "✅ Published successfully!"
```

### Version Bump Script

Create `scripts/bump-version.sh`:
```bash
#!/bin/bash
NEW_VERSION=$1

# Update all version files
sed -i "s/version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
sed -i "s/version=\".*\"/version=\"$NEW_VERSION\"/" python-package/python-pkg/setup.py
sed -i "s/version = \".*\"/version = \"$NEW_VERSION\"/" python-package/python-pkg/pyproject.toml
sed -i "s/__version__ = \".*\"/__version__ = \"$NEW_VERSION\"/" python-package/python-pkg/tempo_cli/__init__.py

# Commit changes
git add .
git commit -m "Bump version to $NEW_VERSION"
git tag "v$NEW_VERSION"
```

## 📞 Support & Issues

If you encounter issues during publishing:

1. **PyPI Issues**: Check [PyPI Help](https://pypi.org/help/)
2. **Crates.io Issues**: See [Cargo Book](https://doc.rust-lang.org/cargo/)
3. **Homebrew Issues**: Visit [Homebrew Docs](https://docs.brew.sh/)

## 🎉 Success!

Once published, users can install Tempo with:

```bash
# Python/UV users
uv install tempo-cli

# Rust users  
cargo install tempo-cli

# Homebrew users
brew install yourusername/tempo/tempo
```

Your time tracking tool is now available worldwide! 🌍