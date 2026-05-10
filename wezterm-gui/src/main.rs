//! wezterm-gui binary entrypoint.
//!
//! Slice 1b.4.b.1: thinned to a binary-edge shim. All meat lives in
//! the wezterm-gui library crate (src/lib.rs); main.rs only handles
//! the binary-init dance that can't / shouldn't move into the lib
//! (panic hook, main-thread designation, config error callback).

#[cfg(feature = "dhat-heap")]
use dhat;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    config::designate_this_as_the_main_thread();
    config::assign_error_callback(mux::connui::show_configuration_error_message);
    wezterm_gui::notify_on_panic();
    if let Err(e) = wezterm_gui::run_cli() {
        wezterm_gui::terminate_with_error(e);
    }
    mux::Mux::shutdown();
    wezterm_gui::shutdown();
}
