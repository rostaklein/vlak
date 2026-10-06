use crate::game::*;
use std::{ffi::c_void, path::PathBuf, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct App {
    game: Game,
    speed: u32,
    help: bool,
    password: Option<String>,
    notice: String,
    progress: PathBuf,
    suppress_char: bool,
    atlas: Option<crate::dos::Atlas>,
    phase: usize,
    language: crate::dos::Language,
    title_screen: bool,
}
impl App {
    fn key(&mut self, key: usize, hwnd: HWND) {
        if self.password.is_some() {
            if key == 27 {
                self.password = None;
            }
            return;
        }
        if key == b'L' as usize {
            self.language = if self.language == crate::dos::Language::Czech {
                crate::dos::Language::English
            } else {
                crate::dos::Language::Czech
            };
            let _ = std::fs::write(
                self.progress.with_file_name("vlak-language.txt"),
                self.language.choose("cz", "en"),
            );
            return;
        }
        if key == 27 {
            unsafe {
                DestroyWindow(hwnd);
            };
            return;
        }
        if key == b'H' as usize {
            self.title_screen = false;
            self.help = !self.help;
            if self.help && self.game.state == State::Playing {
                self.game.state = State::Paused;
            }
            return;
        }
        if self.help {
            if key == 13 || key == 32 {
                self.help = false;
            }
            return;
        }
        if self.title_screen {
            if !matches!(
                key,
                13 | 32 | 37..=40 | 65 | 87 | 68 | 83 | 112 | 115 | 80 | 33 | 34
            ) {
                return;
            }
            self.title_screen = false;
        }
        match key {
            37 | 65 => self.game.turn(Dir::Left),
            38 | 87 => self.game.turn(Dir::Up),
            39 | 68 => self.game.turn(Dir::Right),
            40 | 83 => self.game.turn(Dir::Down),
            82 => self.game.load(self.game.level),
            80 | 115 => {
                self.password = Some(String::new());
                self.suppress_char = key == 80;
                if self.game.state == State::Playing {
                    self.game.state = State::Paused;
                }
            }
            33 => {
                let n = self.game.level.saturating_sub(1);
                self.game.load(n);
            }
            34 => {
                let n = (self.game.level + 1) % self.game.levels.len();
                self.game.load(n);
            }
            112 => {
                let n = self.game.level;
                self.game.load(n);
                if self.game.levels[n].solution.is_empty() {
                    self.notice = "No solution macro in this scene".into();
                } else {
                    self.game.demo = true;
                    self.game.state = State::Playing;
                }
            }
            13 | 32 => match self.game.state {
                State::Ready => self.game.state = State::Playing,
                State::Playing => self.game.state = State::Paused,
                State::Paused => self.game.state = State::Playing,
                State::Crashed => self.game.load(self.game.level),
                State::Won => {
                    let n = (self.game.level + 1) % self.game.levels.len();
                    self.game.load(n);
                    let _ = std::fs::write(&self.progress, n.to_string());
                }
            },
            107 | 187 => {
                self.speed = self.speed.saturating_sub(25).max(65);
                unsafe {
                    SetTimer(hwnd, 1, self.speed, None);
                }
            }
            109 | 189 => {
                self.speed = (self.speed + 25).min(415);
                unsafe {
                    SetTimer(hwnd, 1, self.speed, None);
                }
            }
            _ => {}
        }
    }
    fn character(&mut self, c: u32) {
        if self.suppress_char {
            self.suppress_char = false;
            return;
        }
        let Some(input) = self.password.as_mut() else {
            return;
        };
        if c == 8 {
            input.pop();
            return;
        }
        if c == 13 {
            let value = input.trim().to_uppercase();
            let level = value
                .parse::<usize>()
                .ok()
                .filter(|n| *n >= 1 && *n <= self.game.levels.len())
                .map(|n| n - 1)
                .or_else(|| {
                    self.game
                        .levels
                        .iter()
                        .position(|l| l.name.eq_ignore_ascii_case(&value))
                });
            if let Some(n) = level {
                self.game.load(n);
                self.notice.clear();
                self.password = None;
            } else {
                self.notice = "Unknown scene number or password.".into();
            }
        } else if let Some(ch) = char::from_u32(c) {
            if (ch.is_ascii_alphanumeric() || ch == ' ') && input.len() < 24 {
                input.push(ch.to_ascii_uppercase());
            }
        }
    }
}

// Draw a real 320x200 framebuffer and enlarge whole pixels with GDI.
unsafe fn paint(hwnd: HWND, app: &App) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    let mut area: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut area);
    if area.right <= 0 || area.bottom <= 0 {
        EndPaint(hwnd, &ps);
        return;
    }
    // Clear and draw only into a private bitmap. The visible window receives
    // the complete scene and its black borders in a single final blit.
    let back = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, area.right, area.bottom);
    if back.is_null() || bitmap.is_null() {
        if !bitmap.is_null() {
            DeleteObject(bitmap);
        }
        if !back.is_null() {
            DeleteDC(back);
        }
        EndPaint(hwnd, &ps);
        return;
    }
    let old_bitmap = SelectObject(back, bitmap);
    let brush = CreateSolidBrush(0);
    FillRect(back, &area, brush);
    DeleteObject(brush);
    let frame = if app.title_screen {
        crate::dos::title(
            app.atlas
                .as_ref()
                .expect("title screen requires original assets"),
            app.language,
        )
    } else {
        crate::dos::render(
            &app.game,
            app.atlas.as_ref(),
            app.phase,
            app.help,
            app.password.as_deref(),
            &app.notice,
            app.language,
        )
    };
    let scale = (area.right / 320).min(area.bottom / 200).max(1);
    let (w, h) = (320 * scale, 200 * scale);
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = 320;
    info.bmiHeader.biHeight = -200;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    SetStretchBltMode(back, COLORONCOLOR);
    StretchDIBits(
        back,
        (area.right - w) / 2,
        (area.bottom - h) / 2,
        w,
        h,
        0,
        0,
        320,
        200,
        frame.pixels.as_ptr().cast(),
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    BitBlt(screen, 0, 0, area.right, area.bottom, back, 0, 0, SRCCOPY);
    SelectObject(back, old_bitmap);
    DeleteObject(bitmap);
    DeleteDC(back);
    EndPaint(hwnd, &ps);
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    match msg {
        WM_KEYDOWN => {
            if lp & (1 << 30) == 0 {
                (*ptr).key(wp, hwnd);
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        WM_CHAR => {
            (*ptr).character(wp as u32);
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        WM_TIMER => {
            let mut repaint = false;
            if !(*ptr).help && (*ptr).password.is_none() {
                if (*ptr).game.state == State::Playing {
                    repaint = true;
                    (*ptr).phase = ((*ptr).phase + 1) % 3;
                    (*ptr).game.tick();
                    if (*ptr).game.state == State::Crashed {
                        (*ptr).phase = 0;
                    }
                } else if (*ptr).game.state == State::Crashed && (*ptr).phase < 9 {
                    repaint = true;
                    (*ptr).phase = ((*ptr).phase + 1).min(9);
                }
            }
            if repaint {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_KILLFOCUS => {
            if (*ptr).game.state == State::Playing {
                (*ptr).game.state = State::Paused;
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
            0
        }
        WM_PAINT => {
            paint(hwnd, &*ptr);
            0
        }
        WM_ERASEBKGND => 1,
        WM_GETMINMAXINFO => {
            let limits = &mut *(lp as *mut MINMAXINFO);
            limits.ptMinTrackSize = POINT { x: 336, y: 239 };
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, 1);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

pub fn run() {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let progress = base.join("vlak-progress.txt");
    let atlas = match crate::dos::Atlas::original() {
        Ok(atlas) => Some(atlas),
        Err(e) => {
            asset_error(&e);
            return;
        }
    };
    let preview_title =
        std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--preview-title"));
    if preview_title
        || std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--preview"))
    {
        let path = std::env::args_os()
            .nth(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("vlak-preview.bmp"));
        let requested = std::env::args()
            .nth(3)
            .and_then(|s| s.parse::<usize>().ok());
        let mut game = Game::new(
            parse_scn(include_bytes!("../assets/original-levels.scn"))
                .expect("validated original levels"),
            requested.unwrap_or(1).saturating_sub(1),
        );
        game.demo = true;
        game.state = State::Playing;
        if requested.is_none() {
            for _ in 0..12 {
                game.tick();
            }
        }
        let frame = if preview_title {
            crate::dos::title(atlas.as_ref().unwrap(), crate::dos::Language::Czech)
        } else {
            crate::dos::render(
                &game,
                atlas.as_ref(),
                0,
                false,
                None,
                "",
                crate::dos::Language::Czech,
            )
        };
        if let Err(e) = frame.write_bmp(&path) {
            asset_error(&e.to_string());
        }
        return;
    }
    let custom = std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--custom"));
    let explicit = if custom {
        None
    } else {
        std::env::args_os().nth(1).map(PathBuf::from)
    };
    let scene = if custom {
        None
    } else {
        explicit.clone().or_else(|| {
            [base.join("VLAK.SCN"), PathBuf::from("VLAK.SCN")]
                .into_iter()
                .find(|p| p.exists())
        })
    };
    let levels = if let Some(path) = scene {
        match std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| parse_scn(&b))
        {
            Ok(l) => l,
            Err(e) => {
                unsafe {
                    MessageBoxW(
                        null_mut(),
                        wide(&format!("Cannot load {}:\n{}", path.display(), e)).as_ptr(),
                        wide("Vlak - scene error").as_ptr(),
                        MB_ICONERROR,
                    );
                }
                return;
            }
        }
    } else {
        let embedded = include_bytes!("../assets/original-levels.scn");
        if custom {
            bundled()
        } else {
            match parse_scn(embedded) {
                Ok(levels) => levels,
                Err(e) => {
                    asset_error(&e);
                    return;
                }
            }
        }
    };
    let level = std::fs::read_to_string(&progress)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut app = Box::new(App {
        game: Game::new(levels, level),
        speed: 165,
        help: false,
        password: None,
        notice: String::new(),
        progress,
        suppress_char: false,
        atlas,
        phase: 0,
        title_screen: true,
        language: if std::fs::read_to_string(base.join("vlak-language.txt"))
            .ok()
            .as_deref()
            == Some("en")
        {
            crate::dos::Language::English
        } else {
            crate::dos::Language::Czech
        },
    });
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let name = wide("VlakRustWindow");
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: name.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hIcon: LoadIconW(instance, std::ptr::without_provenance(1)),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&class) == 0 {
            return;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 960,
            bottom: 600,
        };
        AdjustWindowRect(&mut rect, WS_OVERLAPPEDWINDOW, 0);
        let hwnd = CreateWindowExW(
            0,
            name.as_ptr(),
            wide("VLAK").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            null_mut(),
            null_mut(),
            instance,
            (&mut *app as *mut App).cast::<c_void>(),
        );
        if hwnd.is_null() {
            MessageBoxW(
                null_mut(),
                wide("Unable to create game window").as_ptr(),
                name.as_ptr(),
                MB_ICONERROR,
            );
            return;
        }
        SetTimer(hwnd, 1, 165, None);
        let small_icon = LoadImageW(
            instance,
            std::ptr::without_provenance(1),
            IMAGE_ICON,
            16,
            16,
            LR_SHARED,
        );
        if !small_icon.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, small_icon as isize);
        }
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn asset_error(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("Vlak - asset error").as_ptr(),
            MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        App {
            game: Game::new(bundled(), 0),
            speed: 165,
            help: false,
            password: None,
            notice: String::new(),
            progress: PathBuf::new(),
            suppress_char: false,
            atlas: None,
            phase: 0,
            title_screen: false,
            language: crate::dos::Language::Czech,
        }
    }
    #[test]
    fn password_opening_key_is_not_inserted() {
        let mut a = app();
        a.key(80, null_mut());
        a.character('p' as u32);
        assert_eq!(a.password.as_deref(), Some(""));
        a.character('2' as u32);
        a.character(13);
        assert_eq!(a.game.level, 1);
        assert!(a.password.is_none());
    }
    #[test]
    fn f4_does_not_swallow_first_password_character() {
        let mut a = app();
        a.key(115, null_mut());
        a.character('2' as u32);
        a.character(13);
        assert_eq!(a.game.level, 1);
        assert!(a.password.is_none());
    }
    #[test]
    fn space_pauses_and_resumes() {
        let mut a = app();
        a.key(39, null_mut());
        assert_eq!(a.game.state, State::Playing);
        a.key(32, null_mut());
        assert_eq!(a.game.state, State::Paused);
        let head = a.game.train[0];
        a.game.tick();
        assert_eq!(a.game.train[0], head);
        a.key(32, null_mut());
        a.game.tick();
        assert_ne!(a.game.train[0], head);
    }
}
