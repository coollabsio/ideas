use std::{env, fs, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=IDEAS_SKIP_FRONTEND");
    println!("cargo:rerun-if-changed=../../frontend/package.json");
    println!("cargo:rerun-if-changed=../../frontend/bun.lock");
    println!("cargo:rerun-if-changed=../../frontend/src");
    println!("cargo:rerun-if-changed=../../frontend/static");
    println!("cargo:rerun-if-changed=../../frontend/svelte.config.js");
    println!("cargo:rerun-if-changed=../../frontend/vite.config.ts");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let manifest = Path::new(&manifest_dir);
    let frontend = manifest.join("../../frontend");
    let build = frontend.join("build");

    if env::var("IDEAS_SKIP_FRONTEND").as_deref() == Ok("1") {
        fs::create_dir_all(&build).expect("create skipped frontend build dir");
        fs::write(build.join("200.html"), "<div id=app></div>")
            .expect("write placeholder frontend");
        return;
    }

    if deps_need_install(&frontend) {
        run(&frontend, "bun", &["install"]);
    }
    run(&frontend, "bun", &["run", "build"]);
}

fn deps_need_install(frontend: &Path) -> bool {
    let node_modules = frontend.join("node_modules");
    if !node_modules.exists() {
        return true;
    }
    let Ok(node_modules_mtime) = node_modules.metadata().and_then(|m| m.modified()) else {
        return true;
    };
    ["package.json", "bun.lock"]
        .iter()
        .map(|name| frontend.join(name))
        .filter_map(|path| path.metadata().and_then(|m| m.modified()).ok())
        .any(|mtime| mtime > node_modules_mtime)
}

fn run(cwd: &Path, program: &str, args: &[&str]) {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let tmp = Path::new(&out_dir).join("bun-tmp");
    let cache = Path::new(&out_dir).join("bun-cache");
    fs::create_dir_all(&tmp).expect("create bun tmp");
    fs::create_dir_all(&cache).expect("create bun cache");
    let status = Command::new(program)
        .args(args)
        .env("TMPDIR", &tmp)
        .env("BUN_TMPDIR", &tmp)
        .env("BUN_INSTALL_CACHE_DIR", &cache)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {program}: {e}"));
    if !status.success() {
        panic!("{program} {args:?} failed with {status}");
    }
}
