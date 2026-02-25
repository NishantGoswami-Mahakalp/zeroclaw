//! Build script to compile the frontend during cargo build.
//!
//! This follows the Spacedrive approach: build the frontend during cargo build
//! and embed it using rust-embed.

use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=web/src/");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/tsconfig.json");
    println!("cargo:rerun-if-changed=web/tsconfig.app.json");
    println!("cargo:rerun-if-changed=web/tsconfig.node.json");

    if env::var("ZEROCLAW_SKIP_FRONTEND_BUILD").is_ok() {
        println!("Skipping frontend build (ZEROCLAW_SKIP_FRONTEND_BUILD is set)");
        return;
    }

    let web_dir = Path::new("web");

    if !web_dir.exists() {
        println!("web/ directory not found, skipping frontend build");
        return;
    }

    if !web_dir.join("package.json").exists() {
        println!("web/package.json not found, skipping frontend build");
        return;
    }

    let node_modules = web_dir.join("node_modules");
    if !node_modules.exists() {
        println!("node_modules not found, skipping frontend build (run 'bun install' manually)");
        return;
    }

    println!("Building frontend...");

    let output = Command::new("bun")
        .args(["run", "build"])
        .current_dir(web_dir)
        .output()
        .expect("Failed to run 'bun run build'");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Frontend build failed:\n{}", stderr);
        std::process::exit(1);
    }

    let dist_dir = web_dir.join("dist");
    if !dist_dir.exists() {
        eprintln!("Frontend build failed: dist/ directory not found");
        std::process::exit(1);
    }

    if !dist_dir.join("index.html").exists() {
        eprintln!("Frontend build failed: index.html not found in dist/");
        std::process::exit(1);
    }

    println!("Frontend built successfully");
}
