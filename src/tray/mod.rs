use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub const ID_SHOW: &str = "mkac.show";
pub const ID_HIDE: &str = "mkac.hide";
pub const ID_QUIT: &str = "mkac.quit";

static MENU_QUEUE: OnceLock<Mutex<VecDeque<MenuEvent>>> = OnceLock::new();
static TRAY_QUEUE: OnceLock<Mutex<VecDeque<TrayIconEvent>>> = OnceLock::new();

fn menu_queue() -> &'static Mutex<VecDeque<MenuEvent>> {
    MENU_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn tray_queue() -> &'static Mutex<VecDeque<TrayIconEvent>> {
    TRAY_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub struct TrayHandle {
    _icon: TrayIcon,
}

fn make_icon() -> Icon {
    Icon::from_rgba(
        crate::icon::RGBA.to_vec(),
        crate::icon::SIZE,
        crate::icon::SIZE,
    )
    .expect("tray icon")
}

/// Install the tray icon and wire its events to wake egui even when the
/// window is hidden. Without this, clicking "Quit" while the window is
/// minimized-to-tray never reaches `ui()`.
pub fn spawn(ctx: egui::Context) -> anyhow::Result<TrayHandle> {
    let menu = Menu::new();
    menu.append(&MenuItem::with_id(ID_SHOW, "Show MKAC", true, None))?;
    menu.append(&MenuItem::with_id(ID_HIDE, "Hide", true, None))?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&MenuItem::with_id(ID_QUIT, "Quit", true, None))?;

    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("MKAC")
        .with_icon(make_icon())
        .build()?;

    let ctx_menu = ctx.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        menu_queue().lock().unwrap().push_back(event);
        ctx_menu.request_repaint();
    }));

    TrayIconEvent::set_event_handler(Some(move |event| {
        tray_queue().lock().unwrap().push_back(event);
        ctx.request_repaint();
    }));

    Ok(TrayHandle { _icon: icon })
}

pub fn poll_menu() -> Option<MenuEvent> {
    menu_queue().lock().ok()?.pop_front()
}

pub fn poll_tray() -> Option<TrayIconEvent> {
    tray_queue().lock().ok()?.pop_front()
}
