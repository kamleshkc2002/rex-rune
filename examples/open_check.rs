//! Print, for each path given, what Rex Linux decides about offering **Open**: the cheap name and
//! first-bytes check, the desktop's file-type answer, and the verdict that shows the button.
//! With `--click` it also does what a press on Open does (the same checks again, then
//! `xdg-open`), so run that only where `xdg-open` is a stub.
//!
//! `cargo run --locked --no-default-features --example open_check -- [--click] FILE...`
use rex_linux::download::open;
use std::path::Path;

fn main() {
    let mut click = false;
    for arg in std::env::args_os().skip(1) {
        if arg == "--click" {
            click = true;
            continue;
        }
        let path = Path::new(&arg);
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        println!(
            "{}: may_open={} desktop_type={} offer={}",
            path.display(),
            open::may_open(&path),
            open::desktop_type(&path).as_deref().unwrap_or("-"),
            open::may_open_on_desktop(&path),
        );
        if click {
            println!("{}: click={:?}", path.display(), open::open_file(&path));
        }
    }
}
