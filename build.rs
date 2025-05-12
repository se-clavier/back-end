fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    // cxx build for C++ interop
    
    cxx_build::bridge("src/app/algorithm.rs")
        .file("src/cpp/distribute.h")
        .file("src/cpp/mcmf.h")
        .include("src/cpp")
        .flag_if_supported("-std=c++17")
        .compile("cxx-demo");

    // trigger rebuild when Rust bridge or C++ headers change
    println!("cargo:rerun-if-changed=src/app/algorithm/algo.rs");
    println!("cargo:rerun-if-changed=src/cpp/distribute.h");
    println!("cargo:rerun-if-changed=src/cpp/mcmf.h");
}
