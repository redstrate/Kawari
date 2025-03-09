fn main() {
    cc::Build::new()
        .cpp(true)
        .file("src/blowfish/blowfish.cpp")
        .file("src/blowfish/wrapper.cpp")
        .compile("FFXIVBlowfish")
}
