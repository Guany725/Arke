//! Controller lifetime belongs to the window. Only the foreground window accepts actions.

use std::time::{Duration, Instant};

use gpui::{AnyElement, Context, Window, div, prelude::*};
use rust_i18n::t;

use super::Workspace;
use crate::a11y::A11yExt;
use crate::handheld::InterfaceMode;
use crate::handheld_focus::{self, Region};
use crate::handheld_input::{self, Decoder, HandheldAction as H};
use crate::settings::SettingsStore;
use crate::theme;

enum Destination {
    Tab(usize),
    Folder(std::path::PathBuf),
}

const TOOLS: [&str; 8] = [
    "handheld.up",
    "handheld.path",
    "handheld.search",
    "handheld.details",
    "handheld.icons",
    "handheld.columns",
    "handheld.preview",
    "handheld.settings",
];

impl Workspace {
    pub(super) fn sync_handheld(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let store = SettingsStore::global(cx);
        let s = &store.read(cx).settings;
        if s.interface_mode != InterfaceMode::Handheld || !s.controller_enabled {
            self.handheld_task = None;
            self.handheld_region = Region::Files;
            if self.handheld_focus.is_focused(window) {
                self.focus_active(window, cx);
            }
            return;
        }
        if self.handheld_task.is_some() {
            return;
        }
        self.handheld_task = Some(cx.spawn_in(window, async move |this, cx| {
            let start = Instant::now();
            let mut decoder = Decoder::default();
            let mut controller = None;
            let mut delay = Duration::from_millis(16);
            loop {
                cx.background_executor().timer(delay).await;
                let Ok(active) = this.update_in(cx, |_, window, _| window.is_window_active())
                else {
                    break;
                };
                if !active {
                    decoder.reset();
                    delay = Duration::from_millis(250);
                    continue;
                }
                // Some XInput drivers block while discovering pads: keep that off the UI thread.
                let sample = cx
                    .background_executor()
                    .spawn(async move { handheld_input::poll(controller) })
                    .await;
                let Some((id, sample)) = sample else {
                    controller = None;
                    decoder.reset();
                    delay = Duration::from_millis(500);
                    continue;
                };
                if controller != Some(id) {
                    decoder.reset();
                }
                controller = Some(id);
                delay = Duration::from_millis(16);
                if this
                    .update_in(cx, |this, window, cx| {
                        if !window.is_window_active() {
                            decoder.reset();
                            return;
                        }
                        let protected = this.page_shown()
                            || this.handheld_focus.is_focused(window)
                            || this.palette.is_some()
                            || this.net_dialog.is_some()
                            || this.workspaces.is_some()
                            || this.sidebar_menu.is_some()
                            || this.active_view().read(cx).handheld_modal(window, cx);
                        let store = SettingsStore::global(cx);
                        let bindings = &store.read(cx).settings.controller_bindings;
                        let actions = decoder.update_with_bindings(
                            sample,
                            start.elapsed(),
                            bindings,
                            protected,
                        );
                        for action in actions {
                            this.handheld_action(action, window, cx);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    pub(super) fn handheld_action(
        &mut self,
        action: H,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Workspace overlays must never send confirmation to an obscured explorer.
        if self.palette.is_some()
            || self.net_dialog.is_some()
            || self.workspaces.is_some()
            || self.sidebar_menu.is_some()
        {
            if action == H::Back {
                self.dismiss_palette(window, cx);
                self.sidebar_menu = None;
                self.net_dialog = None;
                self.workspaces = None;
                self.focus_active(window, cx);
                cx.notify();
            }
            return;
        }
        let view = self.active_view().clone();
        let region_focused = self.handheld_focus.is_focused(window);
        if !region_focused {
            self.handheld_region = Region::Files;
        }
        let page = self.page_focus(cx).is_some();
        if region_focused || page || !view.read(cx).handheld_modal(window, cx) {
            match action {
                H::PreviousTab => {
                    self.handheld_region = Region::Files;
                    self.prev_tab(&super::PrevTab, window, cx);
                    return;
                }
                H::NextTab => {
                    self.handheld_region = Region::Files;
                    self.next_tab(&super::NextTab, window, cx);
                    return;
                }
                H::FocusRegion => {
                    self.handheld_region = self.handheld_region.next();
                    self.handheld_index = 0;
                    if self.handheld_region == Region::Files {
                        self.focus_active(window, cx);
                    } else {
                        window.focus(&self.handheld_focus, cx);
                    }
                    cx.notify();
                    return;
                }
                H::Settings => {
                    self.open_settings(&super::OpenSettings, window, cx);
                    return;
                }
                _ => (),
            }
        }
        if self.handheld_region != Region::Files {
            let count = if self.handheld_region == Region::Sidebar {
                self.handheld_destinations(cx).len()
            } else {
                TOOLS.len()
            };
            match action {
                H::NavigateUp | H::NavigateLeft | H::PageUp => {
                    self.handheld_index = handheld_focus::step(self.handheld_index, count, false)
                }
                H::NavigateDown | H::NavigateRight | H::PageDown => {
                    self.handheld_index = handheld_focus::step(self.handheld_index, count, true)
                }
                H::Back => {
                    self.handheld_region = Region::Files;
                    self.focus_active(window, cx);
                }
                H::Activate if self.handheld_region == Region::Sidebar => {
                    self.handheld_open_destination(self.handheld_index, window, cx)
                }
                H::Activate => self.handheld_tool(self.handheld_index, window, cx),
                _ => (),
            }
            if self.handheld_region == Region::Sidebar {
                self.handheld_scroll.scroll_to_item(self.handheld_index);
            }
            cx.notify();
            return;
        }
        if page {
            if self.settings_open() {
                use super::settings_panel::{DropdownAccept, DropdownDown, DropdownUp};
                match action {
                    H::NavigateUp => window.dispatch_action(Box::new(DropdownUp), cx),
                    H::NavigateDown => window.dispatch_action(Box::new(DropdownDown), cx),
                    H::Activate => window.dispatch_action(Box::new(DropdownAccept), cx),
                    _ => (),
                }
            }
            if action == H::Back {
                self.close_settings(&super::CloseSettings, window, cx);
                self.close_recycle(window, cx);
                self.close_home(window, cx);
                self.close_whats_new(window, cx);
            }
            return;
        }
        view.update(cx, |e, cx| e.handheld_action(action, window, cx));
        cx.notify();
    }

    pub(super) fn render_handheld_hints(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let store = SettingsStore::global(cx);
        let s = &store.read(cx).settings;
        if s.interface_mode != InterfaceMode::Handheld || !s.controller_hints {
            return None;
        }
        let key = if self.palette.is_some()
            || self.net_dialog.is_some()
            || self.workspaces.is_some()
            || self.sidebar_menu.is_some()
        {
            "handheld.hint_page"
        } else if self.handheld_region != Region::Files && self.handheld_focus.is_focused(window) {
            "handheld.hint_region"
        } else if self.page_shown() {
            "handheld.hint_page"
        } else {
            self.active_view().read(cx).handheld_hint(window, cx)
        };
        let hint = if key == "handheld.hint_files" {
            handheld_input::CONTROLS
                .iter()
                .filter_map(|control| {
                    let action = control.action(&s.controller_bindings);
                    (action != H::Disabled).then(|| {
                        let key = format!("handheld.action.{}", action.key());
                        format!("{} {}", control.label, t!(&key))
                    })
                })
                .collect::<Vec<_>>()
                .join("   ·   ")
        } else {
            t!(key).into_owned()
        };
        Some(
            div()
                .id("handheld-hints")
                .min_h(gpui::px(48.))
                .flex_none()
                .flex()
                .items_center()
                .px(theme::S_16)
                .border_t_1()
                .border_color(theme::line())
                .bg(theme::frame())
                .text_color(theme::text())
                .text_size(theme::FS_14)
                .child(div().w_full().min_w_0().child(hint))
                .into_any_element(),
        )
    }

    fn handheld_destinations(&self, cx: &mut Context<Self>) -> Vec<(Destination, String)> {
        let mut items: Vec<_> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                (
                    Destination::Tab(i),
                    format!("{} · {}", i + 1, tab.view().read(cx).tab_title()),
                )
            })
            .collect();
        let mut paths = std::collections::HashSet::new();
        for (path, label, _) in self.favorites(cx) {
            if paths.insert(path.clone()) {
                items.push((Destination::Folder(path), label));
            }
        }
        let store = SettingsStore::global(cx);
        for path in &store.read(cx).settings.pinned {
            if paths.insert(path.clone()) {
                items.push((
                    Destination::Folder(path.clone()),
                    super::sidebar::display_name(path),
                ));
            }
        }
        for place in self.active_view().read(cx).places.iter() {
            if paths.insert(place.path.clone()) {
                items.push((Destination::Folder(place.path.clone()), place.label.clone()));
            }
        }
        items
    }

    fn handheld_open_destination(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((target, _)) = self.handheld_destinations(cx).into_iter().nth(index) else {
            return;
        };
        self.handheld_region = Region::Files;
        match target {
            Destination::Tab(index) => self.activate(index, window, cx),
            Destination::Folder(path) => {
                self.close_settings(&super::CloseSettings, window, cx);
                self.close_home(window, cx);
                self.close_recycle(window, cx);
                self.close_whats_new(window, cx);
                self.active_view()
                    .clone()
                    .update(cx, |e, cx| e.navigate(path, arke_core::NavKind::New, cx));
                self.focus_active(window, cx);
            }
        }
    }

    fn handheld_tool(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        use crate::explorer::*;
        self.handheld_region = Region::Files;
        self.focus_active(window, cx);
        if index == 7 {
            self.open_settings(&super::OpenSettings, window, cx);
            return;
        }
        if self.page_shown() {
            return;
        }
        let view = self.active_view().clone();
        view.update(cx, |e, cx| match index {
            0 => e.go_up(&GoUp, window, cx),
            1 => e.focus_path_bar(&FocusPathBar, window, cx),
            2 => e.focus_filter(&FocusFilter, window, cx),
            3 => window.dispatch_action(Box::new(ViewDetails), cx),
            4 => window.dispatch_action(Box::new(ViewIcons), cx),
            5 => window.dispatch_action(Box::new(ViewColumns), cx),
            6 => e.handheld_action(H::QuickLook, window, cx),
            _ => (),
        });
    }

    pub(super) fn render_handheld_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let items = self.handheld_destinations(cx);
        div()
            .w(theme::SIDEBAR_W)
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .child(self.render_brand_row(cx))
            .child(
                div()
                    .id("handheld-sidebar")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.handheld_scroll)
                    .px(theme::S_8)
                    .children(items.into_iter().enumerate().map(|(index, (_, label))| {
                        let focused =
                            self.handheld_region == Region::Sidebar && self.handheld_index == index;
                        div()
                            .id(("handheld-place", index))
                            .a11y_button(label.clone())
                            .min_h(gpui::px(48.))
                            .flex()
                            .items_center()
                            .px(theme::S_8)
                            .rounded(theme::R_8)
                            .border_2()
                            .border_color(if focused {
                                theme::signal()
                            } else {
                                theme::clear()
                            })
                            .when(focused, |d| d.bg(theme::signal_wash()))
                            .child(div().truncate().child(label))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.handheld_open_destination(index, window, cx);
                            }))
                    }))
                    .children(self.render_shelf(cx)),
            )
            .into_any_element()
    }

    pub(super) fn render_handheld_toolbar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !crate::handheld::enabled() {
            return None;
        }
        Some(
            div()
                .id("handheld-toolbar")
                .flex()
                .flex_wrap()
                .gap(theme::S_4)
                .px(theme::S_8)
                .children(TOOLS.iter().enumerate().map(|(index, key)| {
                    let focused =
                        self.handheld_region == Region::Toolbar && self.handheld_index == index;
                    div()
                        .id(("handheld-tool", index))
                        .a11y_button(t!(*key).into_owned())
                        .min_h(gpui::px(48.))
                        .min_w(gpui::px(48.))
                        .flex()
                        .items_center()
                        .px(theme::S_12)
                        .rounded(theme::R_8)
                        .border_2()
                        .border_color(if focused {
                            theme::signal()
                        } else {
                            theme::clear()
                        })
                        .when(focused, |d| d.bg(theme::signal_wash()))
                        .child(t!(*key).into_owned())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.handheld_tool(index, window, cx);
                        }))
                }))
                .into_any_element(),
        )
    }
}
