#!/bin/bash

# PyPI Publishing Script for tempo-tracker-cli
# This script handles manual PyPI publishing when GitHub Actions can't complete the task

set -e

echo "🚀 PyPI Publishing Script for tempo-tracker-cli"
echo "==============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}Error: This script must be run from the project root directory${NC}"
    exit 1
fi

# Get current version from Cargo.toml
CARGO_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
echo -e "${BLUE}Current project version: ${GREEN}$CARGO_VERSION${NC}"

# Check if Python package directory exists
PYTHON_PKG_DIR="python-package/python-pkg"
if [[ ! -d "$PYTHON_PKG_DIR" ]]; then
    echo -e "${RED}Error: Python package directory not found at $PYTHON_PKG_DIR${NC}"
    exit 1
fi

# Navigate to Python package directory
cd "$PYTHON_PKG_DIR"

# Check if distribution files exist
if [[ ! -d "dist" ]] || [[ -z "$(ls -A dist 2>/dev/null)" ]]; then
    echo -e "${YELLOW}No distribution files found. Building Python package...${NC}"
    
    # Install build dependencies
    echo -e "${BLUE}Installing build dependencies...${NC}"
    python -m pip install --upgrade pip build twine
    
    # Clean and build
    rm -rf dist/ build/ *.egg-info/
    python -m build
    
    echo -e "${GREEN}✓ Package built successfully${NC}"
else
    echo -e "${GREEN}✓ Distribution files found in dist/${NC}"
fi

# List distribution files
echo -e "${BLUE}Distribution files:${NC}"
ls -la dist/

# Verify package
echo -e "${BLUE}Verifying package integrity...${NC}"
python -m twine check dist/*

if [[ $? -eq 0 ]]; then
    echo -e "${GREEN}✓ Package verification passed${NC}"
else
    echo -e "${RED}✗ Package verification failed${NC}"
    exit 1
fi

# Check for PyPI credentials
echo -e "${BLUE}Checking PyPI credentials...${NC}"

if [[ -z "$TWINE_PASSWORD" ]] && [[ -z "$PYPI_API_TOKEN" ]]; then
    echo -e "${YELLOW}⚠️  No PyPI credentials found in environment variables.${NC}"
    echo -e "${YELLOW}You'll need to provide your PyPI API token when prompted.${NC}"
    echo ""
    echo -e "${BLUE}To get a PyPI API token:${NC}"
    echo "1. Go to https://pypi.org/account/login/"
    echo "2. Log in to your PyPI account"
    echo "3. Go to Account Settings → API tokens"
    echo "4. Create a new API token with scope 'Entire account' or 'Project: tempo-tracker-cli'"
    echo "5. Copy the token (it starts with 'pypi-')"
    echo ""
    echo -e "${BLUE}Then run this script again or set the environment variable:${NC}"
    echo "export TWINE_PASSWORD='your-api-token-here'"
    echo ""
    
    read -p "Do you want to continue and enter the token manually? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Publishing cancelled. Set up your API token and run again.${NC}"
        exit 0
    fi
fi

# Publish to PyPI
echo -e "${BLUE}Publishing to PyPI...${NC}"
echo -e "${YELLOW}Note: Use '__token__' as username and your API token as password${NC}"

# Set up twine configuration for token-based auth if token is provided via environment
if [[ -n "$PYPI_API_TOKEN" ]]; then
    export TWINE_USERNAME="__token__"
    export TWINE_PASSWORD="$PYPI_API_TOKEN"
fi

# Upload to PyPI
python -m twine upload dist/tempo_tracker_cli-$CARGO_VERSION*

if [[ $? -eq 0 ]]; then
    echo -e "${GREEN}🎉 Successfully published tempo-tracker-cli v$CARGO_VERSION to PyPI!${NC}"
    echo -e "${GREEN}Package URL: https://pypi.org/project/tempo-tracker-cli/$CARGO_VERSION/${NC}"
    echo ""
    echo -e "${BLUE}Installation command:${NC}"
    echo -e "${GREEN}pip install tempo-tracker-cli==$CARGO_VERSION${NC}"
    echo ""
    echo -e "${BLUE}Upgrade command:${NC}"
    echo -e "${GREEN}pip install --upgrade tempo-tracker-cli${NC}"
else
    echo -e "${RED}✗ Failed to publish to PyPI${NC}"
    exit 1
fi

# Return to project root
cd - > /dev/null

echo -e "${GREEN}✅ PyPI publishing completed successfully!${NC}"