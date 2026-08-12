fn main() {
    let target = std::env::var("TARGET").expect("Cargo provides TARGET");
    println!("cargo:rustc-env=WEIBIAN_TARGET={target}");
}
