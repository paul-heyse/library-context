fn main() {
    // include_str! tracks edits; the directory tracks additions and removals too.
    println!("cargo:rerun-if-changed=schema");
}
