//! XInput snapshots and deterministic action decoding, independent of GPUI.

use std::time::Duration;

/// Settings store only overrides, so older files inherit defaults for newly added controls.
pub(crate) type Bindings = std::collections::BTreeMap<String, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandheldAction {
    Disabled,
    Activate,
    Back,
    GoParent,
    QuickLook,
    ContextMenu,
    Cut,
    Copy,
    Rename,
    Delete,
    Paste,
    TogglePin,
    AddToShelf,
    RemoveFromShelf,
    ShelfCopyHere,
    ShelfMoveHere,
    ToggleSelection,
    SelectAll,
    ClearSelection,
    NewFolder,
    Refresh,
    FocusPath,
    FocusSearch,
    Settings,
    PreviousTab,
    NextTab,
    PageUp,
    PageDown,
    FocusRegion,
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
}

impl HandheldAction {
    pub(crate) const ALL: &'static [(Self, &'static str)] = &[
        (Self::Disabled, "disabled"),
        (Self::Activate, "open"),
        (Self::Back, "back"),
        (Self::GoParent, "parent"),
        (Self::QuickLook, "preview"),
        (Self::ContextMenu, "menu"),
        (Self::Cut, "cut"),
        (Self::Copy, "copy"),
        (Self::Rename, "rename"),
        (Self::Delete, "delete"),
        (Self::Paste, "paste"),
        (Self::TogglePin, "pin"),
        (Self::AddToShelf, "shelf_add"),
        (Self::RemoveFromShelf, "shelf_remove"),
        (Self::ShelfCopyHere, "shelf_copy_here"),
        (Self::ShelfMoveHere, "shelf_move_here"),
        (Self::ToggleSelection, "select_toggle"),
        (Self::SelectAll, "select_all"),
        (Self::ClearSelection, "select_clear"),
        (Self::NewFolder, "new_folder"),
        (Self::Refresh, "refresh"),
        (Self::FocusPath, "address"),
        (Self::FocusSearch, "search"),
        (Self::Settings, "settings"),
        (Self::PreviousTab, "previous_tab"),
        (Self::NextTab, "next_tab"),
        (Self::PageUp, "page_up"),
        (Self::PageDown, "page_down"),
        (Self::FocusRegion, "region"),
        (Self::NavigateUp, "up"),
        (Self::NavigateDown, "down"),
        (Self::NavigateLeft, "left"),
        (Self::NavigateRight, "right"),
    ];
    pub(crate) fn key(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(action, _)| *action == self)
            .map_or("disabled", |(_, key)| *key)
    }
    pub(crate) fn from_key(key: &str) -> Self {
        Self::ALL
            .iter()
            .find(|(_, k)| *k == key)
            .map_or(Self::Disabled, |(action, _)| *action)
    }
    const fn repeats(self) -> bool {
        matches!(
            self,
            Self::NavigateUp
                | Self::NavigateDown
                | Self::NavigateLeft
                | Self::NavigateRight
                | Self::PageUp
                | Self::PageDown
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Control {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    mask: u32,
    default: HandheldAction,
}

impl Control {
    pub(crate) fn action(self, bindings: &Bindings) -> HandheldAction {
        bindings
            .get(self.key)
            .map_or(self.default, |key| HandheldAction::from_key(key))
    }
}

pub(crate) const CONTROLS: [Control; 12] = [
    Control {
        key: "a",
        label: "A",
        mask: 4096,
        default: HandheldAction::Activate,
    },
    Control {
        key: "b",
        label: "B",
        mask: 8192,
        default: HandheldAction::Back,
    },
    Control {
        key: "x",
        label: "X",
        mask: 16384,
        default: HandheldAction::ContextMenu,
    },
    Control {
        key: "y",
        label: "Y",
        mask: 32768,
        default: HandheldAction::QuickLook,
    },
    Control {
        key: "lb",
        label: "LB",
        mask: 256,
        default: HandheldAction::PreviousTab,
    },
    Control {
        key: "rb",
        label: "RB",
        mask: 512,
        default: HandheldAction::NextTab,
    },
    Control {
        key: "lt",
        label: "LT",
        mask: 65536,
        default: HandheldAction::PageUp,
    },
    Control {
        key: "rt",
        label: "RT",
        mask: 131072,
        default: HandheldAction::PageDown,
    },
    Control {
        key: "menu",
        label: "Menu",
        mask: 16,
        default: HandheldAction::ContextMenu,
    },
    Control {
        key: "view",
        label: "View",
        mask: 32,
        default: HandheldAction::FocusRegion,
    },
    Control {
        key: "ls",
        label: "LS",
        mask: 64,
        default: HandheldAction::Disabled,
    },
    Control {
        key: "rs",
        label: "RS",
        mask: 128,
        default: HandheldAction::Disabled,
    },
];

const NAVIGATION: [Control; 4] = [
    Control {
        key: "up",
        label: "↑",
        mask: 1,
        default: HandheldAction::NavigateUp,
    },
    Control {
        key: "down",
        label: "↓",
        mask: 2,
        default: HandheldAction::NavigateDown,
    },
    Control {
        key: "left",
        label: "←",
        mask: 4,
        default: HandheldAction::NavigateLeft,
    },
    Control {
        key: "right",
        label: "→",
        mask: 8,
        default: HandheldAction::NavigateRight,
    },
];

#[derive(Clone, Copy, Default)]
pub(crate) struct Snapshot {
    pub(crate) buttons: u16,
    pub(crate) left_x: i16,
    pub(crate) left_y: i16,
    pub(crate) left_trigger: u8,
    pub(crate) right_trigger: u8,
}

const UP: u16 = 0x0001;
const DOWN: u16 = 0x0002;
const LEFT: u16 = 0x0004;
const RIGHT: u16 = 0x0008;
#[cfg(test)]
const A: u16 = 0x1000;
#[cfg(test)]
const B: u16 = 0x2000;
const DEADZONE: i32 = 7864;
const INITIAL_DELAY: Duration = Duration::from_millis(280);
const REPEAT: Duration = Duration::from_millis(85);

#[derive(Default)]
pub(crate) struct Decoder {
    previous: u32,
    repeating: Option<(u32, HandheldAction)>,
    next_repeat: Duration,
    armed: bool,
    bindings: Bindings,
    protected: bool,
}

impl Snapshot {
    fn direction(self) -> Option<HandheldAction> {
        use HandheldAction as H;
        let dpad = self.buttons & (UP | DOWN | LEFT | RIGHT);
        // Opposing directions cancel, rather than arbitrarily choosing one.
        match dpad {
            UP => return Some(H::NavigateUp),
            DOWN => return Some(H::NavigateDown),
            LEFT => return Some(H::NavigateLeft),
            RIGHT => return Some(H::NavigateRight),
            0 => (),
            _ => return None,
        }
        let (x, y) = (i32::from(self.left_x), i32::from(self.left_y));
        if x.abs().max(y.abs()) > DEADZONE {
            return Some(if y.abs() >= x.abs() {
                if y > 0 {
                    H::NavigateUp
                } else {
                    H::NavigateDown
                }
            } else if x > 0 {
                H::NavigateRight
            } else {
                H::NavigateLeft
            });
        }
        match (self.left_trigger > 48, self.right_trigger > 48) {
            (true, false) => Some(H::PageUp),
            (false, true) => Some(H::PageDown),
            _ => None,
        }
    }
}

impl Decoder {
    pub(crate) fn reset(&mut self) {
        self.previous = 0;
        self.repeating = None;
        self.armed = false;
    }

    #[cfg(test)]
    fn update(&mut self, state: Snapshot, now: Duration) -> Vec<HandheldAction> {
        self.update_with_bindings(state, now, &Bindings::new(), false)
    }

    pub(crate) fn update_with_bindings(
        &mut self,
        state: Snapshot,
        now: Duration,
        bindings: &Bindings,
        protected: bool,
    ) -> Vec<HandheldAction> {
        if self.bindings != *bindings || self.protected != protected {
            self.reset();
            self.bindings.clone_from(bindings);
            self.protected = protected;
        }
        let mut mask = u32::from(state.buttons & !(UP | DOWN | LEFT | RIGHT));
        let stick = Snapshot {
            left_trigger: 0,
            right_trigger: 0,
            ..state
        };
        if let Some(direction) = stick.direction() {
            mask |= NAVIGATION
                .iter()
                .find(|c| c.default == direction)
                .map_or(0, |c| c.mask);
        }
        // Hysteresis avoids repeated commands when an analog trigger jitters at its threshold.
        if state.left_trigger
            > if self.previous & (1 << 16) != 0 {
                32
            } else {
                48
            }
        {
            mask |= 1 << 16;
        }
        if state.right_trigger
            > if self.previous & (1 << 17) != 0 {
                32
            } else {
                48
            }
        {
            mask |= 1 << 17;
        }
        // Reconnect, focus-context changes and rebinding require release before accepting input.
        if !self.armed {
            self.armed = mask == 0;
            self.previous = mask;
            return Vec::new();
        }
        let pressed = mask & !self.previous;
        self.previous = mask;
        let candidates: Vec<_> = CONTROLS
            .iter()
            .chain(NAVIGATION.iter())
            .filter(|control| pressed & control.mask != 0)
            .map(|control| {
                let fixed = protected || control.mask <= 8;
                (
                    control.mask,
                    if fixed {
                        control.default
                    } else {
                        control.action(bindings)
                    },
                )
            })
            .filter(|(_, action)| *action != HandheldAction::Disabled)
            .collect();
        // Back wins over simultaneous confirmation, even after swapping A and B.
        let command = candidates
            .iter()
            .find(|(_, action)| *action == HandheldAction::Back)
            .or_else(|| candidates.first());
        if let Some(&(control, action)) = command {
            self.repeating = action.repeats().then_some((control, action));
            self.next_repeat = now + INITIAL_DELAY;
            return vec![action];
        }
        if let Some((control, action)) = self.repeating {
            if mask & control == 0 {
                self.repeating = None;
            } else if now >= self.next_repeat {
                // No catch-up bursts after suspension; file operations never enter this branch.
                self.next_repeat = now + REPEAT;
                return vec![action];
            }
        }
        Vec::new()
    }
}

/// Keep the selected controller until it disconnects; never combine different pads.
pub(crate) fn poll(preferred: Option<u32>) -> Option<(u32, Snapshot)> {
    use windows::Win32::UI::Input::XboxController::{XINPUT_STATE, XInputGetState};
    let read = |id| {
        let mut state = XINPUT_STATE::default();
        // SAFETY: XInput writes to a valid, initialized state for a controller index in 0..4.
        if unsafe { XInputGetState(id, &raw mut state) } != 0 {
            return None;
        }
        let pad = state.Gamepad;
        Some((
            id,
            Snapshot {
                buttons: pad.wButtons.0,
                left_x: pad.sThumbLX,
                left_y: pad.sThumbLY,
                left_trigger: pad.bLeftTrigger,
                right_trigger: pad.bRightTrigger,
            },
        ))
    };
    preferred
        .and_then(read)
        .or_else(|| (0..4).filter(|id| Some(*id) != preferred).find_map(read))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_control_accepts_every_action_without_repeating_file_operations() {
        for control in CONTROLS {
            for &(action, key) in HandheldAction::ALL {
                assert_eq!(HandheldAction::from_key(action.key()), action);
                let bindings = Bindings::from([(control.key.to_owned(), key.to_owned())]);
                let mut d = Decoder::default();
                d.update_with_bindings(Snapshot::default(), at(0), &bindings, false);
                let state = Snapshot {
                    buttons: (control.mask & 0xffff) as u16,
                    left_trigger: if control.key == "lt" { 255 } else { 0 },
                    right_trigger: if control.key == "rt" { 255 } else { 0 },
                    ..Snapshot::default()
                };
                let expected = if action == HandheldAction::Disabled {
                    vec![]
                } else {
                    vec![action]
                };
                assert_eq!(
                    d.update_with_bindings(state, at(1), &bindings, false),
                    expected
                );
                if !action.repeats() {
                    assert!(
                        d.update_with_bindings(state, at(10000), &bindings, false)
                            .is_empty()
                    );
                }
            }
        }
    }
    #[test]
    fn rebind_and_dialog_transition_require_release_and_restore_confirmation() {
        let bindings = Bindings::from([("a".to_owned(), "delete".to_owned())]);
        let mut d = armed();
        let held = Snapshot {
            buttons: A,
            ..Snapshot::default()
        };
        assert!(
            d.update_with_bindings(held, at(1), &bindings, false)
                .is_empty()
        );
        d.update_with_bindings(Snapshot::default(), at(2), &bindings, false);
        assert_eq!(
            d.update_with_bindings(held, at(3), &bindings, false),
            [HandheldAction::Delete]
        );
        assert!(
            d.update_with_bindings(held, at(4), &bindings, true)
                .is_empty()
        );
        d.update_with_bindings(Snapshot::default(), at(5), &bindings, true);
        assert_eq!(
            d.update_with_bindings(held, at(6), &bindings, true),
            [HandheldAction::Activate]
        );
        assert!(
            d.update_with_bindings(held, at(7), &bindings, false)
                .is_empty()
        );
    }
    #[test]
    fn trigger_jitter_does_not_repeat_delete() {
        let bindings = Bindings::from([("lt".to_owned(), "delete".to_owned())]);
        let mut d = Decoder::default();
        d.update_with_bindings(Snapshot::default(), at(0), &bindings, false);
        for (i, value) in [255, 49, 47, 48, 50, 33, 255].into_iter().enumerate() {
            let result = d.update_with_bindings(
                Snapshot {
                    left_trigger: value,
                    ..Snapshot::default()
                },
                at(i as u64 + 1),
                &bindings,
                false,
            );
            assert_eq!(
                result,
                if i == 0 {
                    vec![HandheldAction::Delete]
                } else {
                    vec![]
                }
            );
        }
    }
    #[test]
    fn unknown_actions_disable_only_that_control_and_reset_restores_defaults() {
        let bindings = Bindings::from([
            ("a".to_owned(), "future_action".to_owned()),
            ("unknown_button".to_owned(), "delete".to_owned()),
        ]);
        assert_eq!(CONTROLS[0].action(&bindings), HandheldAction::Disabled);
        assert_eq!(CONTROLS[1].action(&bindings), HandheldAction::Back);
        assert_eq!(
            CONTROLS[0].action(&Bindings::new()),
            HandheldAction::Activate
        );
    }
    fn at(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }
    fn armed() -> Decoder {
        let mut d = Decoder::default();
        assert!(d.update(Snapshot::default(), at(0)).is_empty());
        d
    }
    #[test]
    fn repeat_is_delayed_and_never_catches_up() {
        let mut d = armed();
        let down = Snapshot {
            buttons: DOWN,
            ..Snapshot::default()
        };
        assert_eq!(d.update(down, at(1)), [HandheldAction::NavigateDown]);
        assert!(d.update(down, at(280)).is_empty());
        assert_eq!(d.update(down, at(281)), [HandheldAction::NavigateDown]);
        assert_eq!(d.update(down, at(9000)), [HandheldAction::NavigateDown]);
        assert!(d.update(down, at(9001)).is_empty());
    }
    #[test]
    fn buttons_do_not_repeat_and_back_wins() {
        let mut d = armed();
        let a = Snapshot {
            buttons: A,
            ..Snapshot::default()
        };
        assert_eq!(d.update(a, at(1)), [HandheldAction::Activate]);
        assert!(d.update(a, at(5000)).is_empty());
        d.update(Snapshot::default(), at(5001));
        assert_eq!(
            d.update(
                Snapshot {
                    buttons: A | B,
                    ..a
                },
                at(5002)
            ),
            [HandheldAction::Back]
        );
    }
    #[test]
    fn held_input_after_reset_requires_release() {
        let mut d = armed();
        d.reset();
        let a = Snapshot {
            buttons: A,
            ..Snapshot::default()
        };
        assert!(d.update(a, at(1)).is_empty());
        assert!(d.update(a, at(2000)).is_empty());
        d.update(Snapshot::default(), at(2001));
        assert_eq!(d.update(a, at(2002)), [HandheldAction::Activate]);
    }
    #[test]
    fn deadzone_extremes_opposites_and_trigger_cancellation() {
        assert_eq!(
            Snapshot {
                left_x: 7000,
                ..Snapshot::default()
            }
            .direction(),
            None
        );
        assert_eq!(
            Snapshot {
                left_x: i16::MIN,
                ..Snapshot::default()
            }
            .direction(),
            Some(HandheldAction::NavigateLeft)
        );
        assert_eq!(
            Snapshot {
                buttons: UP | DOWN,
                ..Snapshot::default()
            }
            .direction(),
            None
        );
        assert_eq!(
            Snapshot {
                left_trigger: 255,
                right_trigger: 255,
                ..Snapshot::default()
            }
            .direction(),
            None
        );
    }
}
