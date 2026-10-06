use crate::game::*;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 200;
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Language {
    #[default]
    Czech,
    English,
}
impl Language {
    pub fn choose<'a>(self, cz: &'a str, en: &'a str) -> &'a str {
        if self == Self::Czech {
            cz
        } else {
            en
        }
    }
}
// RGB values, matching the sixteen colors of the DOS EGA palette.
pub const EGA: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

pub struct Atlas {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    title: Option<Vec<u32>>,
}
impl Atlas {
    /// The supplied historical sheet is laid out for the paint editor, rather
    /// than in game sprite order. Pack its cells into the game's sprite order.
    pub fn original() -> Result<Self, String> {
        let sheet = decode_png(include_bytes!("../assets/original-sprites.png"), 320, 200)?;
        let title = decode_png(include_bytes!("../assets/original-title.png"), 320, 200)?;
        let extra = decode_png(include_bytes!("../assets/alternate-sprites.png"), 320, 48)?;
        let mut atlas = Self {
            width: 320,
            height: 144,
            pixels: vec![0; 320 * 144],
            title: Some(title.clone()),
        };
        let mut copy = |tile: usize, source: &[u32], x: usize, y: usize| {
            let dx = tile % 20 * 16;
            let dy = tile / 20 * 16;
            for yy in 0..16 {
                for xx in 0..16 {
                    atlas.pixels[(dy + yy) * 320 + dx + xx] = source[(y + yy) * 320 + x + xx];
                }
            }
        };
        for phase in 0..3 {
            for dir in 0..4 {
                let (x, y) = if phase < 2 {
                    ((phase * 4 + dir) * 16, 160)
                } else {
                    (dir * 16, 176)
                };
                copy(1 + phase * 4 + dir, &sheet, x, y);
            }
        }
        for frame in 0..10 {
            copy(13 + frame, &sheet, (8 + frame) * 16, 160);
        }
        // The title reference contains the tiled brick texture at native size.
        copy(23, &title, 0, 176);
        for frame in 0..6 {
            copy(24 + frame, &sheet, (14 + frame) * 16, 176);
        }
        for item in 0..20 {
            for phase in 0..3 {
                if item < 17 {
                    copy(110 + item * 3 + phase, &sheet, item * 16, phase * 16);
                } else {
                    copy(
                        110 + item * 3 + phase,
                        &extra,
                        if item < 19 { item * 16 } else { 0 },
                        0,
                    );
                }
            }
            for dir in 0..4 {
                if item < 17 {
                    copy(30 + item * 4 + dir, &sheet, item * 16, (3 + dir) * 16);
                } else {
                    copy(
                        30 + item * 4 + dir,
                        &extra,
                        if item < 19 { item * 16 } else { 0 },
                        0,
                    );
                }
            }
        }
        Ok(atlas)
    }
    fn sprite(&self, f: &mut Frame, tile: usize, x: i32, y: i32) {
        let columns = self.width / 16;
        let sx = tile % columns * 16;
        let sy = tile / columns * 16;
        if sy + 16 > self.height {
            return;
        }
        for yy in 0..16 {
            for xx in 0..16 {
                f.pixel(
                    x + xx as i32,
                    y + yy as i32,
                    self.pixels[(sy + yy) * self.width + sx + xx],
                );
            }
        }
    }
}

fn decode_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u32>, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    if info.width as usize != width || info.height as usize != height {
        return Err("Unexpected dimensions in original sprite reference".into());
    }
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err("Expected an RGB sprite reference".into()),
    };
    // These clipboard PNGs contain old JPEG artifacts. Restore the documented
    // EGA palette while keeping the original sprite silhouettes and pixels.
    Ok(buffer[..info.buffer_size()]
        .chunks(channels)
        .map(|p| {
            *EGA.iter()
                .min_by_key(|color| {
                    let r = ((*color >> 16) & 255) as i32 - p[0] as i32;
                    let g = ((*color >> 8) & 255) as i32 - p[1] as i32;
                    let b = (*color & 255) as i32 - p[2] as i32;
                    r * r + g * g + b * b
                })
                .unwrap()
        })
        .collect())
}

pub fn title(atlas: &Atlas, lang: Language) -> Frame {
    let mut frame = Frame::new();
    if let Some(pixels) = &atlas.title {
        frame.pixels.clone_from(pixels);
    }
    frame.rect(0, 192, 320, 8, 0);
    frame.text(
        0,
        192,
        lang.choose(
            "ŠIPKY: START  H: POMOC  L: ENGLISH",
            "ARROWS: START  H: HELP  L: ČEŠTINA",
        ),
        EGA[10],
    );
    frame
}

pub struct Frame {
    pub pixels: Vec<u32>,
}
impl Frame {
    fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH * HEIGHT],
        }
    }
    fn pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && x < WIDTH as i32 && y < HEIGHT as i32 {
            self.pixels[y as usize * WIDTH + x as usize] = color;
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.pixel(xx, yy, color);
            }
        }
    }
    fn text(&mut self, x: i32, y: i32, s: &str, color: u32) {
        for (i, ch) in s.chars().enumerate() {
            let ch = ch.to_uppercase().next().unwrap_or(ch);
            let glyph = glyph(ch);
            let accent = match ch {
                'Á' | 'É' | 'Í' | 'Ó' | 'Ú' | 'Ý' => 6,
                'Č' | 'Ď' | 'Ě' | 'Ň' | 'Ř' | 'Š' | 'Ť' | 'Ž' => 10,
                'Ů' => 14,
                _ => 0,
            };
            for col in 0..5 {
                if accent & (16 >> col) != 0 {
                    self.pixel(x + i as i32 * 8 + col + 1, y, color);
                }
            }
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (16 >> col) != 0 {
                        self.pixel(x + i as i32 * 8 + col + 1, y + row as i32 + 1, color);
                    }
                }
            }
        }
    }
    fn panel(&mut self, title: &str, lines: &[&str]) {
        let h = 28 + lines.len() as i32 * 12;
        self.rect(16, 64, 288, h, EGA[1]);
        self.rect(18, 66, 284, h - 4, EGA[15]);
        self.rect(19, 67, 282, h - 6, EGA[1]);
        self.text(28, 74, title, EGA[14]);
        for (i, l) in lines.iter().enumerate() {
            self.text(28, 91 + i as i32 * 12, l, EGA[15]);
        }
    }
    fn fallback_item(&mut self, x: i32, y: i32, t: u8) {
        let shapes: [[u8; 8]; 20] = [
            [0x18, 0x3c, 0x7e, 0xff, 0xff, 0x7e, 0x3c, 0x18],
            [0x81, 0xa5, 0xbd, 0xff, 0x7e, 0x7e, 0x7e, 0],
            [0x18, 0x3c, 0x7e, 0x3c, 0x7e, 0xff, 0x18, 0x18],
            [0x08, 0x10, 0x6e, 0xff, 0xff, 0xff, 0x7e, 0x3c],
            [0x42, 0x66, 0x7e, 0xff, 0xfd, 0x7e, 0x42, 0x42],
            [0x04, 0x18, 0x24, 0x24, 0x66, 0xff, 0xff, 0x66],
            [0, 0x3c, 0x7e, 0xff, 0xdb, 0xff, 0x7e, 0],
            [0x0c, 0x0e, 0x08, 0x08, 0x7f, 0x7f, 0x22, 0x22],
            [0x18, 0x3c, 0x7e, 0x7e, 0x3c, 0x18, 0x18, 0],
            [0x10, 0x10, 0x7e, 0x7e, 0xff, 0xff, 0xff, 0],
            [0x7e, 0x42, 0x5a, 0x42, 0x7e, 0x18, 0xff, 0],
            [0, 0x3c, 0x7e, 0xdb, 0xff, 0xff, 0x42, 0],
            [0x3c, 0x7e, 0xff, 0xff, 0x7e, 0x3c, 0x18, 0x18],
            [0x66, 0x3c, 0x7e, 0xdb, 0xd3, 0x7e, 0x3c, 0x42],
            [0, 0x7e, 0xff, 0xf7, 0xff, 0x7b, 0x52, 0x42],
            [0x7e, 0x7e, 0x3c, 0x18, 0x18, 0x18, 0x7e, 0],
            [0xff, 0x81, 0xbd, 0xa5, 0xbd, 0x81, 0xff, 0],
            [0x18, 0x18, 0x5a, 0xff, 0xff, 0x18, 0x3c, 0x18],
            [0x3c, 0x7e, 0x3c, 0x18, 0x7e, 0x18, 0x3c, 0x66],
            [0x3c, 0x42, 0x99, 0xa5, 0xa5, 0x99, 0x42, 0x3c],
        ];
        let colors = [
            11, 14, 10, 12, 15, 12, 9, 14, 15, 13, 7, 12, 13, 14, 7, 5, 10, 15, 10, 15,
        ];
        let i = (t - b'A').min(19) as usize;
        for (row, bits) in shapes[i].iter().enumerate() {
            for col in 0..8 {
                if bits & (0x80 >> col) != 0 {
                    self.pixel(x + 4 + col, y + 4 + row as i32, EGA[colors[i]]);
                }
            }
        }
    }
    fn wall(&mut self, x: i32, y: i32) {
        self.rect(x, y, 16, 16, EGA[4]);
        for yy in [0, 4, 8, 12] {
            self.rect(x, y + yy, 16, 1, EGA[6]);
            for xx in (if yy % 8 == 0 { 0 } else { 4 }..16).step_by(8) {
                self.rect(x + xx, y + yy, 1, 4, EGA[6]);
                self.rect(x + xx + 1, y + yy + 1, 6, 1, EGA[12]);
            }
        }
    }
    fn engine(&mut self, x: i32, y: i32, d: Dir, phase: usize) {
        // A blue side-view engine with directional cab/chimney placement.
        self.rect(x + 2, y + 6, 12, 6, EGA[1]);
        self.rect(x + 3, y + 6, 10, 2, EGA[9]);
        let cab = if d == Dir::Left { 10 } else { 2 };
        let chimney = if d == Dir::Left { 3 } else { 11 };
        self.rect(x + cab, y + 2, 4, 8, EGA[9]);
        self.rect(x + cab + 1, y + 3, 2, 3, EGA[11]);
        self.rect(x + chimney, y + 3, 2, 4, EGA[7]);
        self.rect(x + chimney - (phase % 2) as i32, y, 3, 2, EGA[15]);
        for xx in [3, 10] {
            self.rect(x + xx, y + 12, 3, 3, EGA[8]);
            self.pixel(x + xx + (phase % 3) as i32, y + 13, EGA[15]);
        }
        if d == Dir::Up {
            self.rect(x + 7, y + 1, 2, 2, EGA[11]);
        } else if d == Dir::Down {
            self.rect(x + 7, y + 14, 2, 2, EGA[11]);
        }
    }
    pub fn write_bmp(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut data = Vec::with_capacity(54 + WIDTH * HEIGHT * 4);
        data.extend(b"BM");
        data.extend(((54 + WIDTH * HEIGHT * 4) as u32).to_le_bytes());
        data.extend([0; 4]);
        data.extend(54u32.to_le_bytes());
        data.extend(40u32.to_le_bytes());
        data.extend((WIDTH as i32).to_le_bytes());
        data.extend((-(HEIGHT as i32)).to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(32u16.to_le_bytes());
        data.extend([0; 24]);
        for color in &self.pixels {
            data.extend(color.to_le_bytes());
        }
        std::fs::write(path, data)
    }
}

pub fn render(
    g: &Game,
    atlas: Option<&Atlas>,
    phase: usize,
    help: bool,
    password: Option<&str>,
    notice: &str,
    lang: Language,
) -> Frame {
    let mut f = Frame::new();
    let crash_frame = phase.min(9);
    let phase = phase % 3;
    for y in 0..H {
        for x in 0..W {
            let t = g.cells[idx(Pos(x, y))];
            let sx = x * 16;
            let sy = y * 16;
            if let Some(a) = atlas {
                let tile = match t {
                    b'#' => 23,
                    b'@' => {
                        if g.remaining == 0 {
                            29
                        } else {
                            24
                        }
                    }
                    b'A'..=b'T' => 110 + (t - b'A') as usize * 3 + phase,
                    _ => 0,
                };
                a.sprite(&mut f, tile, sx, sy);
            } else if t == b'#' {
                f.wall(sx, sy);
            } else if t == b'@' {
                f.rect(sx + 1, sy, 14, 16, EGA[6]);
                f.rect(sx + 2, sy + 1, 12, 14, EGA[14]);
                if g.remaining == 0 {
                    f.rect(sx + 4, sy + 2, 8, 14, 0);
                } else {
                    f.rect(sx + 7, sy + 1, 1, 14, EGA[6]);
                    f.pixel(sx + 6, sy + 8, EGA[6]);
                    f.pixel(sx + 9, sy + 8, EGA[6]);
                }
            } else if matches!(t, b'A'..=b'T') {
                f.fallback_item(sx, sy, t);
            }
        }
    }
    let direction = |d| match d {
        Dir::Left => 0,
        Dir::Up => 1,
        Dir::Right => 2,
        Dir::Down => 3,
    };
    for (i, p) in g.train.iter().enumerate().rev() {
        let x = p.0 * 16;
        let y = p.1 * 16;
        if let Some(a) = atlas {
            let tile = if i == 0 {
                if g.state == State::Crashed {
                    13 + crash_frame
                } else {
                    1 + phase * 4 + direction(g.dir)
                }
            } else {
                let d = Dir::between(*p, g.train[i - 1]);
                30 + (g.cargo.get(i - 1).copied().unwrap_or(b'A') - b'A') as usize * 4
                    + direction(d)
            };
            a.sprite(&mut f, tile, x, y);
        } else if i == 0 {
            f.engine(x, y, g.dir, phase);
        } else {
            f.rect(x + 1, y + 3, 14, 10, EGA[6]);
            f.rect(x + 2, y + 4, 12, 7, EGA[14]);
            f.rect(x + 3, y + 13, 3, 2, EGA[7]);
            f.rect(x + 10, y + 13, 3, 2, EGA[7]);
            f.fallback_item(x, y, g.cargo.get(i - 1).copied().unwrap_or(b'A'));
        }
    }
    f.rect(0, 192, 320, 8, 0);
    f.text(
        0,
        192,
        &format!(
            "{} {:06}",
            lang.choose("SKÓRE", "SCORE"),
            g.cargo.len() * 10
        ),
        EGA[10],
    );
    f.text(136, 192, lang.choose("F4 HESLO", "F4 CODE"), EGA[12]);
    f.text(
        232,
        192,
        &format!("{} {:02}", lang.choose("SCÉNA", "LEVEL"), g.level + 1),
        EGA[10],
    );
    let tr = |cz, en| lang.choose(cz, en);
    if help {
        f.panel(
            tr("VLAK - NÁPOVĚDA", "VLAK - HELP"),
            &[
                tr("ŠIPKY / WASD: ŘÍZENÍ", "ARROWS / WASD: STEER"),
                tr(
                    "SEBER VŠE. POZOR NA VAGÓNY!",
                    "COLLECT ALL. AVOID YOUR TRAIN!",
                ),
                tr("PAK PROJEĎ ZLATOU BRANOU.", "THEN ENTER THE GOLDEN GATE."),
                tr("SPACE: PAUZA  R: ZNOVU", "SPACE: PAUSE  R: RETRY"),
                tr("F1: ŘEŠENÍ  F4: HESLO", "F1: SOLUTION  F4: PASSWORD"),
                tr("PGUP/DN: SCÉNA  +/-: TEMPO", "PGUP/DN: LEVEL  +/-: SPEED"),
                tr("H / ENTER: ZPĚT  ESC: KONEC", "H / ENTER: BACK  ESC: QUIT"),
                tr("L: ENGLISH", "L: ČEŠTINA"),
            ],
        );
    } else if let Some(p) = password {
        f.panel(
            tr("HESLO NEBO ČÍSLO SCÉNY", "PASSWORD OR LEVEL NUMBER"),
            &[
                &format!("> {}_", p),
                tr("ENTER: POTVRDIT  ESC: ZPĚT", "ENTER: CONFIRM  ESC: CANCEL"),
                if notice.is_empty() {
                    ""
                } else {
                    tr("NEZNÁMÉ HESLO / ČÍSLO SCÉNY", "UNKNOWN PASSWORD / LEVEL")
                },
            ],
        );
    } else {
        match g.state {
            State::Ready => f.panel(
                "VLAK",
                &[
                    "(C) MIROSLAV NĚMEČEK 1993",
                    tr("ŠIPKOU VYJEĎ ZE STANICE.", "PRESS AN ARROW TO START."),
                    tr("H: NÁPOVĚDA  F1: ŘEŠENÍ", "H: HELP  F1: SOLUTION"),
                    tr("L: ENGLISH", "L: ČEŠTINA"),
                ],
            ),
            State::Paused => f.panel(
                tr("PAUZA", "PAUSED"),
                &[
                    tr("SPACE / ENTER: POKRAČOVAT", "SPACE / ENTER: CONTINUE"),
                    tr("R: ZNOVU  F4: HESLO", "R: RETRY  F4: PASSWORD"),
                    tr("L: ENGLISH", "L: ČEŠTINA"),
                ],
            ),
            State::Crashed => f.panel(
                tr("BUM! VLAK NABOURAL.", "BOOM! TRAIN CRASHED."),
                &[
                    tr("ENTER / R: ZKUSIT ZNOVU", "ENTER / R: TRY AGAIN"),
                    tr("F1: ŘEŠENÍ SCÉNY", "F1: SHOW SOLUTION"),
                ],
            ),
            State::Won => f.panel(
                if g.level + 1 == g.levels.len() {
                    tr("KONEC HRY - BLAHOPŘEJEME!", "FINISHED - CONGRATULATIONS!")
                } else {
                    tr("VŠECHEN NÁKLAD DORUČEN!", "ALL CARGO DELIVERED!")
                },
                &[
                    &format!(
                        "{}: {}",
                        tr("HESLO", "PASSWORD"),
                        g.levels[(g.level + 1) % g.levels.len()].name
                    ),
                    if g.level + 1 == g.levels.len() {
                        tr("ENTER: HRÁT ZNOVU", "ENTER: PLAY AGAIN")
                    } else {
                        tr("ENTER: DALŠÍ SCÉNA", "ENTER: NEXT LEVEL")
                    },
                ],
            ),
            State::Playing => {}
        }
    }
    f
}

fn glyph(c: char) -> [u8; 7] {
    let c = match c {
        'Á' => 'A',
        'Č' => 'C',
        'Ď' => 'D',
        'É' | 'Ě' => 'E',
        'Í' => 'I',
        'Ň' => 'N',
        'Ó' => 'O',
        'Ř' => 'R',
        'Š' => 'S',
        'Ť' => 'T',
        'Ú' | 'Ů' => 'U',
        'Ý' => 'Y',
        'Ž' => 'Z',
        _ => c,
    };
    match c {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_uses_only_ega_palette() {
        let g = Game::new(bundled(), 0);
        let f = render(&g, None, 0, false, None, "", Language::Czech);
        assert_eq!(f.pixels.len(), 320 * 200);
        assert!(f.pixels.iter().all(|c| EGA.contains(c)));
    }
    #[test]
    fn czech_accented_letters_have_visible_glyphs() {
        for ch in "ÁČĎÉĚÍŇÓŘŠŤÚŮÝŽ".chars() {
            assert_ne!(glyph(ch), [0; 7], "missing {ch}");
        }
    }
    #[test]
    fn original_sprites_decode_and_have_all_game_tiles() {
        let a = Atlas::original().unwrap();
        assert_eq!(a.pixels.len(), 320 * 144);
        assert!(a.pixels.iter().all(|c| EGA.contains(c)));
        for tile in 1..170 {
            let mut f = Frame::new();
            a.sprite(&mut f, tile, 0, 0);
            assert!(f.pixels.iter().any(|c| *c != 0), "empty tile {tile}");
        }
        assert_eq!(title(&a, Language::Czech).pixels.len(), 320 * 200);
    }
}
