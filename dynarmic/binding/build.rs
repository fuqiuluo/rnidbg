use pkg_config;
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn ninja_available() -> bool {
    Command::new("ninja").arg("--version").spawn().is_ok()
}

fn msvc_cmake_tools_available() -> bool {
    Command::new("cmake").arg("--version").spawn().is_ok() && ninja_available()
}

fn setup_env_msvc(compiler: &cc::Tool) {
    // If PATH already contains what we need, skip this
    if msvc_cmake_tools_available() {
        return;
    }

    let target = env::var("TARGET").unwrap();
    let devenv = cc::windows_registry::find_tool(target.as_str(), "devenv");
    let tool_root: PathBuf = match devenv {
        Some(devenv_tool) => devenv_tool.path().parent().unwrap().to_path_buf(),
        None => {
            // if devenv (i.e. Visual Studio) was not found, assume compiler is
            // from standalone Build Tools and look there instead.
            // this should be done properly in cc crate, but for now it's not.
            let tools_name = std::ffi::OsStr::new("BuildTools");
            let compiler_path = compiler.path().to_path_buf();
            compiler_path
                .iter()
                .find(|x| *x == tools_name)
                .expect("Failed to find devenv or Build Tools");
            compiler_path
                .iter()
                .take_while(|x| *x != tools_name)
                .collect::<PathBuf>()
                .join(tools_name)
                .join(r"Common7\IDE")
        }
    };
    let cmake_pkg_dir = tool_root.join(r"CommonExtensions\Microsoft\CMake");
    let cmake_path = cmake_pkg_dir.join(r"CMake\bin\cmake.exe");
    let ninja_path = cmake_pkg_dir.join(r"Ninja\ninja.exe");
    if !cmake_path.is_file() {
        panic!("missing cmake");
    }
    if !ninja_path.is_file() {
        panic!("missing ninja");
    }

    // append cmake and ninja location to PATH
    if let Some(path) = env::var_os("PATH") {
        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
        for tool_path in [cmake_path, ninja_path] {
            paths.push(tool_path.parent().unwrap().to_path_buf());
        }
        let new_path = env::join_paths(paths).unwrap();
        env::set_var("PATH", &new_path);
    }
}

fn build_with_cmake() {
    let uc_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let compiler = cc::Build::new().get_compiler();
    let has_ninja = if compiler.is_like_msvc() {
        setup_env_msvc(&compiler);
        println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
        true
    } else {
        ninja_available()
    };

    // cc crate (as of 1.0.73) misdetects clang as gnu on apple
    if compiler.is_like_gnu() && env::consts::OS != "macos" {
        // see comment on /FORCE:MULTIPLE
        println!("cargo:rustc-link-arg=-Wl,-allow-multiple-definition");
    }

    let mut config = cmake::Config::new(&uc_dir);
    if has_ninja {
        config.generator("Ninja");
    }

    let dst = config
        .no_build_target(true)
        .define("BUILD_TESTING", "OFF")
        .define("MASTER_PROJECT", "OFF")
        .define("DYNARMIC_USE_BUNDLED_EXTERNALS", "OFF")
        .define("DYNARMIC_USE_LLVM", "OFF")
        .define("CMAKE_BUILD_TYPE", "Release")
        .build();
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").display()
    );

    // Lazymio(@wtdcode): Dynamic link may break. See: https://github.com/rust-lang/cargo/issues/5077
    println!("cargo:rustc-link-lib=static=dynarmic");
    if !compiler.is_like_msvc() {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=fmt");

        // if is x86_64
        if cfg!(target_arch = "x86_64") {
            println!("cargo:rustc-link-lib=Zydis");
        }
    }
}

fn main() {
    match pkg_config::Config::new()
        .atleast_version("2")
        .cargo_metadata(false)
        .probe("dynarmic")
    {
        Ok(lib) => {
            for dir in lib.link_paths {
                println!("cargo:rustc-link-search=native={}", dir.to_str().unwrap());
            }
            println!("cargo:rustc-link-arg=-Wl,-allow-multiple-definition");
            println!("cargo:rustc-link-lib=static=dynarmic");
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=fmt");
            println!("cargo:rustc-link-lib=m");

            if cfg!(target_arch = "x86_64") {
                println!("cargo:rustc-link-lib=Zydis");
            }
        }
        Err(_) => {
            println!("pkg-config could not find dynarmic, building from source");
            build_with_cmake();
        }
    };
}