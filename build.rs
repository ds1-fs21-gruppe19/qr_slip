fn main() {
    #[cfg(windows)]
    println!("cargo:rustc-link-search=C:\\Program Files\\wkhtmltopdf\\lib\\");
}
