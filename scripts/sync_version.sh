#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🔄 Starting version synchronization...${NC}"

# Get version from Cargo.toml
VERSION=$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
    echo -e "${RED}❌ Error: Could not extract version from Cargo.toml${NC}"
    exit 1
fi

echo -e "${GREEN}📦 Syncing version ${VERSION} to Python package...${NC}"

# Function to update file with proper error handling
update_file() {
    local file=$1
    local pattern=$2
    local replacement=$3
    local description=$4
    
    if [ -f "$file" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "$pattern" "$file"
        else
            sed -i "$pattern" "$file"
        fi
        echo -e "${GREEN}✅ Updated $description${NC}"
    else
        echo -e "${YELLOW}⚠️  Warning: $file not found, skipping...${NC}"
    fi
}

# Update pyproject.toml
update_file "python-package/python-pkg/pyproject.toml" \
           "s/^version = \".*\"/version = \"$VERSION\"/" \
           "pyproject.toml"

# Update setup.py
update_file "python-package/python-pkg/setup.py" \
           "s/version=\".*\"/version=\"$VERSION\"/" \
           "setup.py"

# Update any other version files that might exist
if [ -f "VERSION" ]; then
    echo "$VERSION" > VERSION
    echo -e "${GREEN}✅ Updated VERSION file${NC}"
fi

echo -e "${GREEN}🎉 Version sync complete! All files updated to version ${VERSION}${NC}"
