# Setting Up Crates.io Publishing

To enable automatic publishing to crates.io, you need to set up a CARGO_REGISTRY_TOKEN secret in your GitHub repository.

## Step 1: Get a Crates.io API Token

1. Go to [crates.io](https://crates.io/) and sign in with your GitHub account
2. Navigate to your account settings: https://crates.io/settings/tokens
3. Click "New Token"
4. Give it a name like "tempo-cli-github-actions"
5. Copy the generated token (it will only be shown once)

## Step 2: Add the Token as a GitHub Secret

1. Go to your GitHub repository: https://github.com/own-path/vibe
2. Navigate to Settings > Secrets and variables > Actions
3. Click "New repository secret"
4. Name: `CARGO_REGISTRY_TOKEN`
5. Value: Paste the token from crates.io
6. Click "Add secret"

## Step 3: Test the Setup

Once you've added the secret, the next commit to main will automatically:

1. ✅ Detect version changes in Cargo.toml
2. ✅ Publish to crates.io with the new version
3. ✅ Create a GitHub release with the same version
4. ✅ Sync versions across all package files
5. ✅ Create git tags

## Important Notes

- Make sure you're listed as an owner or maintainer of the `tempo-cli` crate on crates.io
- If the crate doesn't exist yet, you'll need to publish it manually once first:
  ```bash
  cargo publish
  ```
- After the initial publish, all subsequent releases will be automatic

## Security

- The CARGO_REGISTRY_TOKEN is stored securely in GitHub Secrets
- It's only accessible to your GitHub Actions workflows
- Never commit the token to your repository