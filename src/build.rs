fn main() {
    println!("cargo:rustc-link-search=native=/usr/lib/libX11.so.6.4.0");
    println!("cargo:rustc-link-lib=x11");
}
