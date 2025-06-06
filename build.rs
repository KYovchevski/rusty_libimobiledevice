// jkcoxson

extern crate bindgen;

use std::{env, fs::canonicalize, path::PathBuf};

fn main() {
    // Tell cargo to invalidate the built crate whenever build files change
    println!("cargo:rerun-if-changed=wrapper.h");
    // println!("cargo:rerun-if-changed=build.rs");

    ////////////////////////////
    //   BINDGEN GENERATION   //
    ////////////////////////////

    if cfg!(feature = "pls-generate") {
        // Get gnutls path per OS
        // let gnutls_path = match env::consts::OS {
        //     "linux" => "/usr/include",
        //     "macos" => "/opt/homebrew/include",
        //     "windows" => {
        //         panic!("Generating bindings on Windows is broken, pls remove the pls-generate feature.");
        //     }
        //     _ => panic!("Unsupported OS"),
        // };

        let bindings = bindgen::Builder::default()
            // The input header we would like to generate
            // bindings for.
            .header("wrapper.h")
            .clang_arg(format!("-D{}", "LIBIMOBILEDEVICE_STATIC"))
            .clang_arg(format!("-D{}", "HAVE_OPENSSL"))
            .clang_arg(format!("-I{}", "libimobiledevice/include/"))
            .clang_arg(format!("-I{}", "libimobiledevice"))
            .clang_arg(format!("-I{}", "libplist/include"))
            // Include in clang build
            // Tell cargo to invalidate the built crate whenever any of the
            // included header files changed.
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            // Finish the builder and generate the bindings.
            .generate()
            // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file("src/bindings.rs")
            .expect("Couldn't write bindings!");
        // panic!("Hecknah");
    }

    if cfg!(feature = "vendored") {
        println!("cargo:rerun-if-changed=libimobiledevice/common/");

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

        let lib_path = out_path.join("lib");
        if !lib_path.exists() {
            // Create lib directory
            std::fs::create_dir(&lib_path).unwrap();
        }
        let lib_path = lib_path.canonicalize().unwrap().display().to_string();

        // println!("cargo:rustc-link-search=native=C:/Code/rusty_libimobiledevice/target/debug/build/tlsimple-113936cecf6b9453/out");
        // link_lib("mbedtlsmono");

        println!("cargo:rustc-link-search=native={}", lib_path);

        build_libimobiledevice_glue(&lib_path);
        build_libtatsu(&lib_path);
        build_libusbmuxd(&lib_path);

        // build_tfa_psa_crypto(&lib_path);
        // build_mbedtls(&lib_path);

        build_libimobiledevice(&lib_path);

        // Change current directory to OUT_DIR

        // Lib path setup

        // Set the LD_LIBRARY_PATH environment variable to lib_path

        // Include path setup
        let include_path = out_path.join("include");
        if !include_path.exists() {
            // Create include directory
            std::fs::create_dir(&include_path).unwrap();
        }
        let include_path = include_path.canonicalize().unwrap().display().to_string();
        // Search for where openssl-src placed my libs
        env::set_current_dir("../../").unwrap();

        let mut c_flags = vec![];
        let mut cxx_flags = vec![];

        // If building for Windows, set the env var for mbedtls
        if env::var("TARGET").unwrap().contains("windows") {
            env::set_var("WINDOWS_BUILD", "1");

            // Windows needs extra crap
            println!("cargo:rustc-link-lib=dylib=iphlpapi");
            println!("cargo:rustc-link-lib=dylib=shell32");
            println!("cargo:rustc-link-lib=dylib=ole32");
        }
        if env::var("TARGET").unwrap().contains("apple") {
            println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.13");
            c_flags.push("-mmacosx-version-min=10.13".to_string());
            cxx_flags.push("-mmacosx-version-min=10.13".to_string());
        }
        c_flags.push(format!("-L{} -I{}", lib_path, include_path));

        // Build those bad bois

        // We shouldn't need to build libplist because plist_plus already does that for us.... right?
        // let mut dst = autotools::Config::new("libplist");
        // let mut dst = dst.without("cython", None);
        // for flag in &c_flags {
        //     dst = dst.cflag(flag);
        // }
        // for flag in &cxx_flags {
        //     dst = dst.cxxflag(flag);
        // }
        // let dst = dst.build();

        // println!(
        //     "cargo:rustc-link-search=native={}",
        //     dst.join("lib").display()
        // );
    } else {
        // Check if folder ./override exists
        let override_path = PathBuf::from("./override").join(env::var("TARGET").unwrap());
        if override_path.exists() {
            println!(
                "cargo:rustc-link-search={}",
                canonicalize(&override_path).unwrap().display()
            );
        }

        println!("cargo:rustc-link-search=/usr/local/lib");
        println!("cargo:rustc-link-search=/usr/lib");
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=/usr/local/opt/libimobiledevice/lib");
        println!("cargo:rustc-link-search=/usr/local/opt/libusbmuxd/lib");
        println!("cargo:rustc-link-search=/usr/local/opt/libimobiledevice-glue/lib");
    }

    // Link libi* deps
    // println!(
    //     "cargo:rustc-link-lib={}=imobiledevice-1.0",
    //     location_determinator
    // );
    // println!("cargo:rustc-link-lib={}=usbmuxd-2.0", location_determinator);
    // println!(
    //     "cargo:rustc-link-lib={}=imobiledevice-glue-1.0",
    //     location_determinator
    // );
    // println!("cargo:rustc-link-lib={}=plist-2.0", location_determinator);

    // println!("cargo:rustc-link-lib={location_determinator}=crypto");
    // println!("cargo:rustc-link-lib={location_determinator}=ssl");
}

fn filter_source_files(path: PathBuf) -> std::io::Result<impl Iterator<Item = PathBuf>> {
    let iter = std::fs::read_dir(path)?.filter_map(|f| {
        f.ok().and_then(|f| {
            let path = f.path();
            let extension = path.extension().and_then(|ext| ext.to_str());

            if extension == Some("cpp") || extension == Some("c") {
                println!("cargo::rerun-if-changed={}", path.display());

                Some(path)
            } else {
                None
            }
        })
    });

    Ok(iter)
}

fn link_lib(lib_name: &str) {
    let location_determinator = if cfg!(feature = "static") || cfg!(feature = "vendored") {
        "static"
    } else {
        "dylib"
    };

    println!("cargo:rustc-link-lib={location_determinator}={lib_name}");
}

fn build_libimobiledevice_glue(out_dir: &String) {
    let version = std::process::Command::new("sh")
        .arg("./libimobiledevice-glue/git-version-gen")
        .output()
        .expect("Failed to determine libimobiledevice-glue version");

    let version =
        str::from_utf8(&version.stdout).expect("Could not process git-version-gen output");
    let version = format!("\"{}\"", version);

    let mut build = cc::Build::new();
    build.define("LIMD_GLUE_STATIC", None);
    build.define("LIMD_GLUE_API", None);

    for f in filter_source_files("libimobiledevice-glue/src/".into())
        .expect("Failed to find source files for libimobiledevice-glue")
    {
        build.file(f);
    }

    build.include("libimobiledevice-glue/include");
    build.include("libplist/include");
    build.define("PACKAGE_VERSION", version.as_str());
    build.out_dir(out_dir);

    build.compile("imobiledevice-glue");

    link_lib("imobiledevice-glue");
    // panic!();
    // let mut dst = autotools::Config::new("libimobiledevice-glue");
    // let dst = dst.without("cython", None);
    // let mut dst = dst.env("PKG_CONFIG_PATH", out_path.join("lib/pkgconfig"));
    // for flag in &c_flags {
    //     dst = dst.cflag(flag);
    // }
    // for flag in &cxx_flags {
    //     dst = dst.cxxflag(flag);
    // }
    // let dst = dst.build();

    // println!("cargo:rustc-link-search=native={}", dst.display());

    // let mut dst = autotools::Config::new("libusbmuxd");
    // let dst = dst.without("cython", None);
    // let mut dst = dst.env("PKG_CONFIG_PATH", out_path.join("lib/pkgconfig"));
    // for flag in &c_flags {
    //     dst = dst.cflag(flag);
    // }
    // for flag in &cxx_flags {
    //     dst = dst.cxxflag(flag);
    // }
    // let dst = dst.build();

    // println!(
    //     "cargo:rustc-link-search=native={}",
    //     dst.join("lib").display()
    // );
}

fn build_libtatsu(out_dir: &String) {
    let version = std::process::Command::new("sh")
        .arg("./libtatsu/git-version-gen")
        .output()
        .expect("Failed to determine libtatsu version");

    let version =
        str::from_utf8(&version.stdout).expect("Could not process git-version-gen output");
    let version = format!("\"{}\"", version);

    let mut build = cc::Build::new();

    for f in filter_source_files("libtatsu/src/".into())
        .expect("Failed to find source files for libtatsu")
    {
        build.file(f);
    }

    build.include("libtatsu/include");
    build.include("curl/include");
    build.include("libplist/include");
    build.define("PACKAGE_VERSION", version.as_str());

    build.out_dir(out_dir);

    build.compile("libtatsu");

    link_lib("libtatsu");

    // let mut dst = autotools::Config::new("libtatsu");
    // let dst = dst.without("cython", None);
    // let mut dst = dst.env("PKG_CONFIG_PATH", out_path.join("lib/pkgconfig"));
    // for flag in &c_flags {
    //     dst = dst.cflag(flag);
    // }
    // for flag in &cxx_flags {
    //     dst = dst.cxxflag(flag);
    // }
    // let dst = dst.build();

    // println!(
    //     "cargo:rustc-link-search=native={}",
    //     dst.join("lib").display()
    // );
}

fn build_libimobiledevice(out_dir: &String) {
    let version = std::process::Command::new("sh")
        .arg("./libimobiledevice/git-version-gen")
        .output()
        .expect("Failed to determine libimobiledevice version");

    let version =
        str::from_utf8(&version.stdout).expect("Could not process git-version-gen output");
    let version = format!("\"{}\"", version);

    // dbg!(env!("CC"));

    let mut build = cc::Build::new();
    build.define("LIMD_GLUE_API", Some(""));
    build.define("LIMD_GLUE_STATIC", Some(""));
    build.define("USBMUXD_API", Some(""));
    build.define("LIBUSBMUXD_STATIC", Some(""));
    build.define("LIBIMOBILEDEVICE_API", Some(""));
    build.define("LIBIMOBILEDEVICE_STATIC", Some(""));

    for f in filter_source_files("libimobiledevice/src/".into())
        .and_then(|iter| {
            filter_source_files("libimobiledevice/common/".into())
                .map(|other_iter| iter.chain(other_iter))
        })
        .expect("Failed to find source files for libimobiledevice")
    {
        build.file(f);
    }

    build.include("libimobiledevice/include");
    build.include("libimobiledevice");
    build.include("curl/include");
    build.include("libplist/include");
    build.include("dirent/include");
    build.include("libusbmuxd/include");

    build.include("mbedtls/include");
    build.include("mbedtls/TF-PSA-Crypto/include");
    build.include("mbedtls/TF-PSA-Crypto/drivers/builtin/include");
    build.include("libimobiledevice-glue/include");

    build.define("PACKAGE_VERSION", version.as_str());
    build.define("HAVE_SYS_TYPES_H", None);
    build.define("HAVE_RUSTLS", None);

    build.out_dir(out_dir);

    build.compile("libimobiledevice");

    link_lib("libimobiledevice");

    // let mut dst = autotools::Config::new("libimobiledevice");
    // let dst = dst.without("cython", None);
    // let mut dst = dst.env("PKG_CONFIG_PATH", out_path.join("lib/pkgconfig"));
    // for flag in &c_flags {
    //     dst = dst.cflag(flag);
    // }
    // for flag in &cxx_flags {
    //     dst = dst.cxxflag(flag);
    // }
    // let dst = dst.build();

    // println!(
    //     "cargo:rustc-link-search=native={}",
    //     dst.join("lib").display()
    // );
}

// fn build_tfa_psa_crypto(out_dir: &String) {
//     let mut build = cc::Build::new();

//     let core_src = filter_source_files("mbedtls/tf-psa-crypto/core/".into()).expect("Failed to find source files for tf-psa-crypto");
//     let builtin_drivers_src = filter_source_files("mbedtls/tf-psa-crypto/drivers/builtin/src/".into()).expect("Failed to find source files for tf-psa-crypto");

//     for f in core_src.chain(builtin_drivers_src) {
//         build.file(f);
//     }

//     build.define("MBEDTLS_CONFIG_FILE", "<c:/code/rusty_libimobiledevice/mbedtls/tf-psa-crypto/scripts/stuff.jasper>");

//     build.include("mbedtls/tf-psa-crypto/core");
//     build.include("mbedtls/tf-psa-crypto/include");
//     build.include("mbedtls/tf-psa-crypto/drivers/builtin/include");
//     build.include("mbedtls/tf-psa-crypto/drivers/builtin/src");
//     // build.define("PACKAGE_VERSION", version.as_str());

//     build.out_dir(out_dir);

//     build.compile("tf_psa_crypto");
//     link_lib("tf_psa_crypto");
// }

// fn build_mbedtls(out_dir: &String) {
//     let mut build = cc::Build::new();

//     build.define("MBEDTLS_ALLOW_PRIVATE_ACCESS", Some("1"));
//     build.define("MBEDTLS_PK_C", Some("1"));
//     build.define("MBEDTLS_DEBUG_C", Some("1"));

//     for f in filter_source_files("mbedtls/library/".into())
//         .expect("Failed to find source files for mbedtls")
//     {
//         build.file(f);
//     }

//     build.include("mbedtls/include");
//     build.include("mbedtls/tf-psa-crypto/core");
//     build.include("mbedtls/tf-psa-crypto/include");
//     build.include("mbedtls/tf-psa-crypto/drivers/builtin/include");
//     build.include("mbedtls/tf-psa-crypto/drivers/builtin/src");

//     build.out_dir(out_dir);

//     build.compile("mbedtls");
//     link_lib("mbedtls");
// }

fn build_libusbmuxd(out_dir: &String) {
    let version = std::process::Command::new("sh")
        .arg("./libusbmuxd/git-version-gen")
        .output()
        .expect("Failed to determine libusbmuxd version");

    let version =
        str::from_utf8(&version.stdout).expect("Could not process git-version-gen output");
    let version = format!("\"{}\"", version);

    let mut build = cc::Build::new();
    build.define("USBMUXD_API", Some(""));
    build.define("LIBUSBMUXD_STATIC", Some(""));
    build.define("LIMD_GLUE_API", Some(""));
    build.define("LIMD_GLUE_STATIC", Some(""));

    for f in filter_source_files("libusbmuxd/src/".into())
        .expect("Failed to find source files for libusbmuxd")
    {
        build.file(f);
    }

    build.include("libusbmuxd/include");
    build.include("libplist/include");
    build.include("libimobiledevice-glue/include");

    build.define("PACKAGE_VERSION", version.as_str());

    build.out_dir(out_dir);

    build.compile("libusbmuxd");

    link_lib("libusbmuxd");
}

// fn repo_setup(url: &str) {
//     let mut cmd = std::process::Command::new("git");
//     cmd.arg("clone");
//     cmd.arg("--depth=1");
//     cmd.arg(url);
//     cmd.output().unwrap();
//     env::set_current_dir(url.split('/').last().unwrap().replace(".git", "")).unwrap();
//     env::set_var("NOCONFIGURE", "1");
//     let mut cmd = std::process::Command::new("./autogen.sh");
//     let _ = cmd.output();
//     env::remove_var("NOCONFIGURE");
//     env::set_current_dir("..").unwrap();
// }
