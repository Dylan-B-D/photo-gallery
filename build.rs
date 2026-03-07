use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn npm_executable() -> &'static str {
    if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }
}

fn run_npm(current_dir: &PathBuf, args: &[&str]) {
    let status = Command::new(npm_executable())
        .args(args)
        .current_dir(current_dir)
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "Failed to run {:?} {:?} (cwd: {}): {e}",
                npm_executable(),
                args,
                current_dir.display()
            )
        });

    if !status.success() {
        panic!(
            "Command {:?} {:?} failed with status {status} (cwd: {})",
            npm_executable(),
            args,
            current_dir.display()
        );
    }
}

fn main() {
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=package-lock.json");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=postcss.config.js");
    println!("cargo:rerun-if-changed=src/tailwind.css");
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let build_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|| "0".to_string());
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_timestamp}");

    let git_describe = Command::new("git")
        .args(["describe", "--always", "--dirty", "--tags"])
        .current_dir(&manifest_dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_DESCRIBE={git_describe}");

    let git_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&manifest_dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_SHA={git_sha}");

    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!("cargo:rustc-env=BUILD_PROFILE={profile}");
    let force = std::env::var("FORCE_TAILWIND_BUILD").ok().as_deref() == Some("1");
    let skip = std::env::var("SKIP_TAILWIND_BUILD").ok().as_deref() == Some("1");

    if skip {
        return;
    }

    if profile != "release" && !force {
        return;
    }

    let static_css_dir = manifest_dir.join("static").join("css");
    let tailwind_css_path = static_css_dir.join("tailwind.css");
    let node_modules_dir = manifest_dir.join("node_modules");

    std::fs::create_dir_all(&static_css_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to create static css directory {}: {e}",
            static_css_dir.display()
        )
    });

    if !node_modules_dir.exists() {
        run_npm(&manifest_dir, &["ci"]);
    }

    let tailwind_bin = node_modules_dir.join(".bin").join(if cfg!(windows) {
        "tailwindcss.cmd"
    } else {
        "tailwindcss"
    });

    if !tailwind_bin.exists() {
        run_npm(&manifest_dir, &["ci"]);
    }

    run_npm(&manifest_dir, &["run", "build"]);

    if !tailwind_css_path.exists() {
        panic!(
            "Tailwind build completed, but {} was not created",
            tailwind_css_path.display()
        );
    }
}
