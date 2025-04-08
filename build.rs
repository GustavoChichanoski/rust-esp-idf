fn main() {
    // println!("cargo:rustc-link-arg=-Tlinkall.x");
    // println!("cargo:rustc-link-arg=-nostartfiles");
    // println!("cargo:rustc-link-arg=-Trom_functions.x");
    // println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-cfg=esp32");
}
