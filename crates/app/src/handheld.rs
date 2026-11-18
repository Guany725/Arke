//! Shared interface mode and logical-pixel metrics. Never changes stored desktop density.

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{Pixels, px};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InterfaceMode {
    #[default]
    Desktop,
    Handheld,
}

static ENABLED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_mode(mode: InterfaceMode) -> bool {
    let enabled = mode == InterfaceMode::Handheld;
    ENABLED.swap(enabled, Ordering::Relaxed) != enabled
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn metric(desktop: Pixels) -> Pixels {
    if enabled() {
        desktop.max(px(48.))
    } else {
        desktop
    }
}

pub(crate) fn row_metric(desktop: Pixels) -> Pixels {
    if enabled() {
        desktop.max(px(48.))
    } else {
        desktop
    }
}

/// Ask Windows to show its own touch keyboard for this desktop window.
pub(crate) fn show_keyboard(window: &gpui::Window) {
    use windows::UI::ViewManagement::InputPane;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::IInputPaneInterop;
    let Some(owner) = crate::explorer::Explorer::owner_from(window) else {
        return;
    };
    let result = (|| -> windows::core::Result<bool> {
        let factory = windows::core::factory::<InputPane, IInputPaneInterop>()?;
        // SAFETY: the HWND belongs to the live GPUI window on its UI thread.
        let pane: InputPane = unsafe { factory.GetForWindow(HWND(owner as *mut _)) }?;
        pane.TryShow()
    })();
    if let Err(error) = result {
        log::debug!("Touch keyboard unavailable: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::InterfaceMode;

    #[test]
    fn mode_round_trips_and_old_settings_keep_desktop() {
        let old: crate::settings::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.interface_mode, InterfaceMode::Desktop);
        assert!(old.controller_bindings.is_empty());
        let custom: crate::settings::Settings = serde_json::from_str(
            r#"{"interface_mode":"handheld","controller_bindings":{"a":"copy","rt":"shelf_add","ls":"future_action"}}"#,
        ).unwrap();
        let restored: crate::settings::Settings =
            serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(restored.controller_bindings, custom.controller_bindings);
        assert_eq!(restored.interface_mode, InterfaceMode::Handheld);
        for (mode, text) in [
            (InterfaceMode::Desktop, "\"desktop\""),
            (InterfaceMode::Handheld, "\"handheld\""),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), text);
            assert_eq!(serde_json::from_str::<InterfaceMode>(text).unwrap(), mode);
        }
    }
}
