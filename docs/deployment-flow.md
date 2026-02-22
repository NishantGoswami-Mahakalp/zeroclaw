# ZeroClaw CI/CD Flow for Forks
#
# This document describes the deployment workflow for this fork.
#
# BRANCH STRATEGY:
# - main: Stable releases only
# - Feature branches: PRs for testing
#
# DEPLOYMENT FLOW:
#
# 1. Development/Testing:
#    - Create feature branch from main
#    - Open PR to main
#    - CI runs: tests, lint, build
#
# 2. Staging (Manual):
#    - workflow_dispatch: Select "staging" environment
#    - Deploys to staging VPS for testing
#
# 3. Production Release:
#    - Create git tag: git tag v1.0.0
#    - Push tag: git push origin v1.0.0
#    - Auto-deploys to production VPS
#
# ENVIRONMENT SETUP:
#
# Required Secrets (GitHub → Repository → Settings → Secrets):
# - VPS_HOST: Your VPS hostname/IP
# - VPS_USER: SSH username (e.g., root, ubuntu)
# - VPS_SSH_KEY: Private SSH key with VPS access
#
# MANUAL DEPLOYMENT:
#
# Option 1: Via GitHub UI
#   1. Go to Actions → Deploy to VPS
#   2. Click "Run workflow"
#   3. Select environment (staging/production)
#   4. Click "Run workflow"
#
# Option 2: Via CLI
#   gh workflow run deploy-vps.yml -f environment=staging
#
# VERSION TAGGING:
#
# To deploy a specific version:
#   git tag v1.0.0
#   git push origin v1.0.0
#
# Tag format: v<major>.<minor>.<patch>
#   - v1.0.0, v1.0.1, v2.0.0, etc.
#
# VALIDATION BEFORE DEPLOYMENT:
#
# 1. Run tests locally:
#    cargo test --locked
#    cargo fmt --all -- --check
#    cargo clippy --all-targets -- -D clippy::correctness
#
# 2. Build Docker locally:
#    docker build -t zeroclaw:test .
#
# 3. Test manually:
#    docker run -p 42617:42617 zeroclaw:test
#
# 4. Only then create tag:
#    git tag v1.0.0
#    git push origin v1.0.0
#
# ROLLBACK:
#
# To rollback to previous version:
#   git tag -d v1.0.1
#   git push origin :refs/tags/v1.0.1
#   # Then redeploy previous tag:
#   # gh workflow run deploy-vps.yml -f environment=production -f image_tag=v1.0.0
