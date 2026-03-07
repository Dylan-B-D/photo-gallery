use std::path::PathBuf;
use std::process::Command;

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

    let profile = std::env::var("PROFILE").unwrap_or_default();
    let force = std::env::var("FORCE_TAILWIND_BUILD").ok().as_deref() == Some("1");
    let skip = std::env::var("SKIP_TAILWIND_BUILD").ok().as_deref() == Some("1");

    if skip {
        return;
    }

    if profile != "release" && !force {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
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
