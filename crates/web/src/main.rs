mod components;
#[cfg(debug_assertions)]
mod gallery;
mod shell;

use leptos::mount::mount_to_body;

fn main() {
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    if leptos::prelude::window()
        .location()
        .pathname()
        .is_ok_and(|path| path.starts_with("/gallery"))
    {
        mount_to_body(gallery::Gallery);
        return;
    }
    mount_to_body(shell::AppShell);
}
