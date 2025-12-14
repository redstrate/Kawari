fn main() {
    // Embed migrations as they change
    println!("cargo:rerun-if-changed=migrations");
}
