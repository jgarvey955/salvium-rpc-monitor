use std::ffi::{CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::mem::zeroed;
use std::ptr::{null, null_mut};

#[repr(C)]
pub struct Display {
    _private: [u8; 0],
}

pub type Window = c_ulong;
pub type Drawable = c_ulong;
pub type Atom = c_ulong;
pub type Time = c_ulong;
pub type KeySym = c_ulong;
pub type GC = *mut c_void;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XAnyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: Window,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XExposeEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub count: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XConfigureEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub above: Window,
    pub override_redirect: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XKeyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub keycode: c_uint,
    pub same_screen: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XButtonEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub button: c_uint,
    pub same_screen: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ClientMessageData {
    pub b: [c_char; 20],
    pub s: [i16; 10],
    pub l: [c_long; 5],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XClientMessageEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: Window,
    pub message_type: Atom,
    pub format: c_int,
    pub data: ClientMessageData,
}

#[repr(C)]
pub union XEvent {
    pub type_: c_int,
    pub xany: XAnyEvent,
    pub xexpose: XExposeEvent,
    pub xconfigure: XConfigureEvent,
    pub xkey: XKeyEvent,
    pub xbutton: XButtonEvent,
    pub xclient: XClientMessageEvent,
    pub pad: [c_long; 24],
}

pub const KEY_PRESS: c_int = 2;
pub const BUTTON_PRESS: c_int = 4;
pub const EXPOSE: c_int = 12;
pub const CONFIGURE_NOTIFY: c_int = 22;
pub const CLIENT_MESSAGE: c_int = 33;

pub const KEY_PRESS_MASK: c_long = 1;
pub const BUTTON_PRESS_MASK: c_long = 1 << 2;
pub const EXPOSURE_MASK: c_long = 1 << 15;
pub const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XRootWindow(display: *mut Display, screen_number: c_int) -> Window;
    fn XBlackPixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    fn XWhitePixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    fn XCreateSimpleWindow(
        display: *mut Display,
        parent: Window,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        border_width: c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> Window;
    fn XStoreName(display: *mut Display, w: Window, window_name: *const c_char) -> c_int;
    fn XSelectInput(display: *mut Display, w: Window, event_mask: c_long) -> c_int;
    fn XMapWindow(display: *mut Display, w: Window) -> c_int;
    fn XCreateGC(
        display: *mut Display,
        d: Drawable,
        valuemask: c_ulong,
        values: *mut c_void,
    ) -> GC;
    fn XSetForeground(display: *mut Display, gc: GC, foreground: c_ulong) -> c_int;
    fn XFillRectangle(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XDrawRectangle(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XDrawString(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        string: *const c_char,
        length: c_int,
    ) -> c_int;
    fn XClearWindow(display: *mut Display, w: Window) -> c_int;
    fn XNextEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    fn XLookupString(
        event_struct: *mut XKeyEvent,
        buffer_return: *mut c_char,
        bytes_buffer: c_int,
        keysym_return: *mut KeySym,
        status_in_out: *mut c_void,
    ) -> c_int;
    fn XInternAtom(
        display: *mut Display,
        atom_name: *const c_char,
        only_if_exists: c_int,
    ) -> Atom;
    fn XSetWMProtocols(
        display: *mut Display,
        w: Window,
        protocols: *mut Atom,
        count: c_int,
    ) -> c_int;
    fn XFlush(display: *mut Display) -> c_int;
    fn XFreeGC(display: *mut Display, gc: GC);
    fn XDestroyWindow(display: *mut Display, w: Window) -> c_int;
    fn XCloseDisplay(display: *mut Display) -> c_int;
}

pub struct X11App {
    display: *mut Display,
    window: Window,
    gc: GC,
    black: c_ulong,
    white: c_ulong,
    wm_delete_window: Atom,
    width: u32,
    height: u32,
}

impl X11App {
    pub fn open(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let display = unsafe { XOpenDisplay(null()) };
        if display.is_null() {
            return Err("unable to open X display; start from a desktop session".to_string());
        }

        let screen = unsafe { XDefaultScreen(display) };
        let root = unsafe { XRootWindow(display, screen) };
        let black = unsafe { XBlackPixel(display, screen) };
        let white = unsafe { XWhitePixel(display, screen) };

        let window = unsafe {
            XCreateSimpleWindow(display, root, 80, 80, width, height, 1, black, white)
        };

        let title = CString::new(title).map_err(|_| "window title contains NUL".to_string())?;
        unsafe {
            XStoreName(display, window, title.as_ptr());
            XSelectInput(
                display,
                window,
                EXPOSURE_MASK | KEY_PRESS_MASK | BUTTON_PRESS_MASK | STRUCTURE_NOTIFY_MASK,
            );
            XMapWindow(display, window);
        }

        let gc = unsafe { XCreateGC(display, window, 0, null_mut()) };

        let wm_delete_name =
            CString::new("WM_DELETE_WINDOW").map_err(|_| "invalid atom name".to_string())?;
        let wm_delete_window = unsafe { XInternAtom(display, wm_delete_name.as_ptr(), 0) };
        let mut protocols = [wm_delete_window];
        unsafe {
            XSetWMProtocols(display, window, protocols.as_mut_ptr(), 1);
            XFlush(display);
        }

        Ok(Self {
            display,
            window,
            gc,
            black,
            white,
            wm_delete_window,
            width,
            height,
        })
    }

    pub fn run<F>(&mut self, state: &mut crate::app::AppState, mut draw: F)
    where
        F: FnMut(&Self, &crate::app::AppState),
    {
        draw(self, state);
        loop {
            let mut event: XEvent = unsafe { zeroed() };
            unsafe {
                XNextEvent(self.display, &mut event);
            }

            let event_type = unsafe { event.type_ };
            match event_type {
                EXPOSE => {
                    if unsafe { event.xexpose.count } == 0 {
                        draw(self, state);
                    }
                }
                CONFIGURE_NOTIFY => {
                    let cfg = unsafe { event.xconfigure };
                    self.width = cfg.width.max(1) as u32;
                    self.height = cfg.height.max(1) as u32;
                    draw(self, state);
                }
                KEY_PRESS => {
                    let mut key_event = unsafe { event.xkey };
                    let mut buffer = [0i8; 8];
                    let mut keysym = 0;
                    let len = unsafe {
                        XLookupString(
                            &mut key_event,
                            buffer.as_mut_ptr(),
                            buffer.len() as c_int,
                            &mut keysym,
                            null_mut(),
                        )
                    };
                    if len > 0 {
                        let c = buffer[0] as u8;
                        if c == b'q' {
                            break;
                        }
                        if c == b'j' || c == b'J' {
                            let content = state.content();
                            let visible = ((self.height as i32 - 50) / 20).max(1) as usize;
                            state.scroll(1, content.len(), visible);
                            draw(self, state);
                        }
                        if c == b'k' || c == b'K' {
                            let content = state.content();
                            let visible = ((self.height as i32 - 50) / 20).max(1) as usize;
                            state.scroll(-1, content.len(), visible);
                            draw(self, state);
                        }
                    }
                }
                BUTTON_PRESS => {
                    let btn = unsafe { event.xbutton };
                    let x = btn.x as i32;
                    let y = btn.y as i32;
                    const MENU_HEIGHT: i32 = 30;
                    const LIST_WIDTH: i32 = 200;
                    const ITEM_HEIGHT: i32 = 20;
                    if y >= MENU_HEIGHT && x < LIST_WIDTH {
                        let item_index = ((y - MENU_HEIGHT) / ITEM_HEIGHT) as usize;
                        state.set_view(item_index);
                        draw(self, state);
                    }
                }
                CLIENT_MESSAGE => {
                    let client = unsafe { event.xclient };
                    let message = unsafe { client.data.l[0] as Atom };
                    if message == self.wm_delete_window {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn draw_shell(&self, state: &crate::app::AppState) {
        unsafe {
            XClearWindow(self.display, self.window);
            XSetForeground(self.display, self.gc, self.white);
            XFillRectangle(self.display, self.window, self.gc, 0, 0, self.width, self.height);

            XSetForeground(self.display, self.gc, self.black);

            // Draw menu bar
            XDrawRectangle(self.display, self.window, self.gc, 0, 0, self.width, 30);
            self.draw_text(10, 20, "File  (q=quit, j/k=scroll)");

            // Draw list
            let items = state.list_items();
            let mut y = 50;
            for (i, item) in items.iter().enumerate() {
                let selected = state.is_view_selected(i);
                let prefix = if selected { "* " } else { "  " };
                self.draw_text(10, y, &format!("{}{}", prefix, item));
                y += 20;
            }

            // Draw content (scrollable)
            let content = state.content();
            let visible_lines = ((self.height as i32 - 50) / 20).max(1) as usize;
            let start = state.scroll_offset().min(content.len());
            let end = (start + visible_lines).min(content.len());
            let mut y = 50;
            for line in content.iter().take(end).skip(start) {
                self.draw_text(220, y, line);
                y += 20;
            }

            // Scroll indicator / slider
            let total = content.len();
            let indicator = format!("{}/{}", start + 1, total);
            self.draw_text(220, self.height as i32 - 10, &indicator);
            let slider = state.scroll_bar(total, visible_lines);
            self.draw_text(340, self.height as i32 - 10, &slider);

            XFlush(self.display);
        }
    }

    fn draw_text(&self, x: i32, y: i32, text: &str) {
        let sanitized: String = text
            .chars()
            .map(|ch| if ch.is_ascii() { ch } else { '?' })
            .collect();
        if let Ok(text) = CString::new(sanitized) {
            unsafe {
                XDrawString(
                    self.display,
                    self.window,
                    self.gc,
                    x,
                    y,
                    text.as_ptr(),
                    text.as_bytes().len() as c_int,
                );
            }
        }
    }
}

impl Drop for X11App {
    fn drop(&mut self) {
        unsafe {
            if !self.gc.is_null() {
                XFreeGC(self.display, self.gc);
            }
            XDestroyWindow(self.display, self.window);
            XCloseDisplay(self.display);
        }
    }
}
