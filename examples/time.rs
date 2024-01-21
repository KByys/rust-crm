fn main() {
    let path = "/home/pesju/Downloads/XBYDriver.AppImage";
    std::process::Command::new(path).spawn().unwrap();
}