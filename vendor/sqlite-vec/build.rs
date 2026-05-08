fn main() {
    // The upstream `sqlite-vec.c` file includes optional companion C units
    // (diskann + rescore) behind macros that default to `1`. The crates.io
    // package omits those extra files, so we explicitly disable them.
    cc::Build::new()
        .file("sqlite-vec.c")
        .define("SQLITE_CORE", None)
        .define("SQLITE_VEC_ENABLE_DISKANN", Some("0"))
        .define("SQLITE_VEC_ENABLE_RESCORE", Some("0"))
        .warnings(false)
        .compile("sqlite_vec0");
}
