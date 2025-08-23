fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-arg-bin=interception-installer=/MANIFEST:EMBED");
        println!(
            r"cargo:rustc-link-arg-bin=interception-installer=/MANIFESTUAC:level='requireAdministrator'"
        );
    }
}
