fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_manifest_file("manifest.xml");
    res.compile().unwrap();
}
