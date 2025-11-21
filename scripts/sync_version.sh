#!/bin/bash
set -e

# Get version from Cargo.toml
VERSION=$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)
echo "Syncing version $VERSION to Python package..."

# Update pyproject.toml
if [ -f "python-package/python-pkg/pyproject.toml" ]; then
    # Use sed to replace version. Assumes format: version = "x.y.z"
    # macOS sed requires empty string for -i
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" python-package/python-pkg/pyproject.toml
    else
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" python-package/python-pkg/pyproject.toml
    fi
    echo "Updated pyproject.toml"
fi

# Update setup.py
if [ -f "python-package/python-pkg/setup.py" ]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/version=\".*\"/version=\"$VERSION\"/" python-package/python-pkg/setup.py
    else
        sed -i "s/version=\".*\"/version=\"$VERSION\"/" python-package/python-pkg/setup.py
    fi
    echo "Updated setup.py"
fi

echo "Version sync complete!"
