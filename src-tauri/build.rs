fn main() {
    // The icon is compiled into the executable's resources; without this, swapping the
    // icon files keeps shipping the previously embedded one.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
