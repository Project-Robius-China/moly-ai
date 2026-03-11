pub mod add_provider_modal;
#[cfg(not(target_arch = "wasm32"))]
pub mod botfather_view;
pub mod moly_server_screen;
pub mod provider_view;
pub mod providers;
pub mod providers_screen;
pub mod sync_modal;
pub mod utilities_modal;
use makepad_widgets::Cx;

pub fn live_design(cx: &mut Cx) {
    providers_screen::live_design(cx);
    moly_server_screen::live_design(cx);
    #[cfg(not(target_arch = "wasm32"))]
    botfather_view::live_design(cx);
    provider_view::live_design(cx);
    providers::live_design(cx);
    add_provider_modal::live_design(cx);
    sync_modal::live_design(cx);
    utilities_modal::live_design(cx);
}
