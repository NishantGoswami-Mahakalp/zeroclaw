# ZeroClaw Fork - Deployment Guide

## Overview

This is a **fork** of ZeroClaw with custom changes. Deployment builds from **your fork's current state**, not from upstream.

## Deployment Flow

```
Your Changes → Run Workflow → Build from Fork → Deploy to VPS
```

## Quick Deploy

```bash
# Go to GitHub → Actions → Deploy to VPS
# Click "Run workflow"
# Select environment: production
# Click "Run workflow"
```

The workflow will:
1. **Build** Docker image from your fork's current code
2. **Push** to GitHub Container Registry (`ghcr.io`)
3. **Deploy** to your VPS

## No Tags Needed

Just run the workflow - it builds from your fork's current state:
- Uses commit SHA as image tag
- No risk of upstream overwriting your changes

## Required Secrets

Add in GitHub → Repo Settings → Secrets:

| Secret | Description | Example |
|--------|-------------|---------|
| `VPS_HOST` | VPS IP or hostname | `192.168.1.100` |
| `VPS_USER` | SSH username | `root` or `ubuntu` |
| `VPS_SSH_KEY` | Private SSH key | `-----BEGIN RSA...` |

### Setting up SSH Key

1. Generate a deploy key:
   ```bash
   ssh-keygen -t ed25519 -C "deploy@zeroclaw" -f deploy_key
   ```

2. Add **public key** to VPS `~/.ssh/authorized_keys`:
   ```bash
   cat deploy_key.pub >> ~/.ssh/authorized_keys
   ```

3. Add **private key** to GitHub Secrets:
   - Copy: `cat deploy_key`
   - Add as secret `VPS_SSH_KEY`

## Validation Checklist

Before deploying, verify locally:

```bash
# 1. Run tests
cargo test --locked

# 2. Format check
cargo fmt --all -- --check

# 3. Lint
cargo clippy --all-targets -- -D clippy::correctness

# 4. Build locally
docker build -t zeroclaw:test .

# 5. Quick test
docker run -p 42617:42617 zeroclaw:test
```

## Manual VPS Access

```bash
# SSH to VPS
ssh user@your-vps-ip

# Check container
docker logs zeroclaw

# Check health
curl http://localhost:42617/health

# Restart
docker restart zeroclaw
```

## Rollback

To rollback, re-run the workflow with the previous commit:
- Not easily supported (would need to store previous image tags)
- For quick fix: SSH to VPS and run previous image manually
