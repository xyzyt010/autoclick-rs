fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/autoclick-rs.ico");
        res.set_manifest_file("assets/autoclick-rs.exe.manifest");
        res.set("FileDescription", "AutoClick-RS - Cross-platform automatic key presser");
        res.set("ProductName", "AutoClick-RS");
        res.set("CompanyName", "xyzyt010");
        res.compile().unwrap();
    }
}
