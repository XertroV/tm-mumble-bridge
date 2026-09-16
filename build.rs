use std::io;
use winresource::WindowsResource;

fn main() -> io::Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    Ok(())
}

