use std::io;
use winresource::WindowsResource;

fn main() -> io::Result<()> {
    if cfg!(target_os = "windows") {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
