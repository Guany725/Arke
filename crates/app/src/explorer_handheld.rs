//! Handheld actions reuse the existing file, menu and preview operations.

use gpui::{App, Context, Focusable, Window};

use super::{
    Explorer, GoUp, MoveLeft, MoveRight, OpenSelected, SelectNext, SelectPageDown, SelectPageUp,
    SelectPrev, ViewMode,
};
use crate::handheld_input::HandheldAction as H;

impl Explorer {
    pub(super) fn start_handheld_press(
        &mut self,
        index: usize,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handheld_press = None;
        self.handheld_press_opened = false;
        self.handheld_press_origin = position;
        if !crate::handheld::enabled() {
            return;
        }
        let path = self.current_path().map(std::path::Path::to_path_buf);
        self.handheld_press = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(550))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                let delta = window.mouse_position() - position;
                if !crate::handheld::enabled()
                    || !window.is_window_active()
                    || cx.has_active_drag()
                    || delta.x.abs() > gpui::px(8.)
                    || delta.y.abs() > gpui::px(8.)
                    || this.current_path() != path.as_deref()
                    || this.current_cursor() != Some(index)
                    || this.renaming.is_some()
                    || this.context_menu.is_some()
                {
                    return;
                }
                this.rename_wait = None;
                this.rename_press = None;
                this.marquee = None;
                this.handheld_press_opened = true;
                this.open_context_menu(position, window, cx);
            });
        }));
    }

    pub(crate) fn handheld_modal(&self, window: &Window, cx: &App) -> bool {
        self.context_menu.is_some()
            || self.view_options_open
            || self.confirm.is_some()
            || self.quick_look
            || self.renaming.is_some()
            || self.editing_path
            || self
                .filter_input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
            || !self.list_focus.is_focused(window)
    }

    pub(crate) fn handheld_hint(&self, window: &Window, cx: &App) -> &'static str {
        if self.confirm.is_some() || self.context_menu.is_some() {
            "handheld.hint_menu"
        } else if self.quick_look {
            "handheld.hint_preview"
        } else if self.handheld_modal(window, cx) {
            "handheld.hint_input"
        } else {
            "handheld.hint_files"
        }
    }

    pub(crate) fn handheld_action(
        &mut self,
        action: H,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Dialogs and editable text own confirmation. Never open a file underneath them.
        if self.view_options_open {
            if action == H::Back {
                self.close_view_options(cx);
            }
            return;
        }
        if self.confirm.is_some() {
            match action {
                H::Activate => window.dispatch_action(Box::new(super::confirm::ConfirmOk), cx),
                H::Back => window.dispatch_action(Box::new(super::confirm::ConfirmCancel), cx),
                _ => (),
            }
            return;
        }
        if self.context_menu.is_some() {
            use super::menu::*;
            match action {
                H::NavigateUp | H::PageUp => self.menu_up(&MenuUp, window, cx),
                H::NavigateDown | H::PageDown => self.menu_down(&MenuDown, window, cx),
                H::NavigateLeft => self.menu_left(&MenuLeft, window, cx),
                H::NavigateRight => self.menu_right(&MenuRight, window, cx),
                H::Activate => self.menu_confirm(&MenuConfirm, window, cx),
                H::Back => self.menu_dismiss(&MenuDismiss, window, cx),
                _ => (),
            }
            return;
        }
        if self.renaming.is_some()
            || self.editing_path
            || self
                .filter_input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        {
            match action {
                H::Activate => window.dispatch_action(Box::new(crate::text_input::Confirm), cx),
                H::Back => window.dispatch_action(Box::new(crate::text_input::Cancel), cx),
                _ => (),
            }
            return;
        }
        if self.quick_look {
            match action {
                H::Back | H::QuickLook => self.close_quick_look(cx),
                H::NavigateLeft => self.move_left(&MoveLeft, window, cx),
                H::NavigateRight => self.move_right(&MoveRight, window, cx),
                _ => (),
            }
            return;
        }
        // Return focus without running a file action underneath a popover.
        if !self.list_focus.is_focused(window) {
            if action == H::Back {
                self.close_view_options(cx);
                self.cancel_path_edit(window, cx);
            }
            return;
        }
        match action {
            H::NavigateUp => self.select_prev(&SelectPrev, window, cx),
            H::NavigateDown => self.select_next(&SelectNext, window, cx),
            H::PageUp => self.select_page_up(&SelectPageUp, window, cx),
            H::PageDown => self.select_page_down(&SelectPageDown, window, cx),
            H::NavigateLeft if self.view_mode == ViewMode::Details => self.go_up(&GoUp, window, cx),
            H::NavigateLeft => self.move_left(&MoveLeft, window, cx),
            H::NavigateRight if self.view_mode == ViewMode::Details => {
                if let Some(ix) = self.current_cursor()
                    && self.path_at(ix).is_some_and(|(_, is_dir)| is_dir)
                {
                    self.open_index(ix, cx);
                }
            }
            H::NavigateRight => self.move_right(&MoveRight, window, cx),
            H::Activate => self.open_selected(&OpenSelected, window, cx),
            H::Back => {
                if !self.close_view_options(cx) {
                    self.go_up(&GoUp, window, cx);
                }
            }
            H::ContextMenu => {
                self.open_context_menu_action(&super::menu::OpenContextMenu, window, cx);
                self.menu_down(&super::menu::MenuDown, window, cx);
            }
            H::QuickLook => self.toggle_quick_look(cx),
            H::GoParent => self.go_up(&GoUp, window, cx),
            H::Cut => self.cut_files(&super::CutFiles, window, cx),
            H::Copy => self.copy_files(&super::CopyFiles, window, cx),
            H::Rename => self.rename_file(&super::RenameFile, window, cx),
            H::Delete => self.delete_files(&super::DeleteFiles, window, cx),
            H::Paste => self.paste_files(&super::PasteFiles, window, cx),
            H::TogglePin => self.toggle_pin_targets(&self.shelf_targets(), cx),
            H::AddToShelf => self.add_to_shelf(&super::AddToShelf, window, cx),
            H::RemoveFromShelf => self.remove_selection_from_shelf(cx),
            H::ShelfCopyHere => self.shelf_to_here(false, cx),
            H::ShelfMoveHere => self.shelf_to_here(true, cx),
            H::ToggleSelection => self.toggle_cursor(&super::ToggleCursor, window, cx),
            H::SelectAll => self.select_all(&super::SelectAll, window, cx),
            H::ClearSelection => {
                self.selection.clear();
                cx.notify();
            }
            H::NewFolder => self.new_folder(&super::NewFolder, window, cx),
            H::Refresh => self.refresh(&super::Refresh, window, cx),
            H::FocusPath => self.focus_path_bar(&super::FocusPathBar, window, cx),
            H::FocusSearch => self.focus_filter(&super::FocusFilter, window, cx),
            _ => (),
        }
    }
}
