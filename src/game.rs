use std::collections::VecDeque;

pub const W: i32 = 20;
pub const H: i32 = 12;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos(pub i32, pub i32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}
impl Dir {
    pub fn step(self, p: Pos) -> Pos {
        match self {
            Self::Up => Pos(p.0, p.1 - 1),
            Self::Down => Pos(p.0, p.1 + 1),
            Self::Left => Pos(p.0 - 1, p.1),
            Self::Right => Pos(p.0 + 1, p.1),
        }
    }
    pub fn between(a: Pos, b: Pos) -> Self {
        if b.0 > a.0 {
            Self::Right
        } else if b.0 < a.0 {
            Self::Left
        } else if b.1 > a.1 {
            Self::Down
        } else {
            Self::Up
        }
    }
}
#[derive(Clone)]
pub struct Level {
    pub name: String,
    pub cells: [u8; 240],
    pub start: Pos,
    pub solution: Vec<Dir>,
}
pub fn idx(p: Pos) -> usize {
    (p.1 * W + p.0) as usize
}
pub fn inside(p: Pos) -> bool {
    p.0 >= 0 && p.0 < W && p.1 >= 0 && p.1 < H
}

// Each bundled board has a verified non-self-intersecting route. The walls and
// cargo vary; the route is also available as an optional demonstration.
pub fn bundled() -> Vec<Level> {
    let names = [
        "FIRST TRAIN",
        "ORCHARD",
        "FREIGHT",
        "SWITCHBACK",
        "DIAMONDS",
        "LONG HAUL",
        "NIGHT SHIFT",
        "ZIG ZAG",
        "EXPRESS",
        "CARGO YARD",
        "FINAL RUN",
        "HOME STATION",
    ];
    names
        .iter()
        .enumerate()
        .map(|(n, name)| {
            let mut route = Vec::new();
            if n % 2 == 0 {
                for row in 0..5 {
                    let y = 1 + row * 2;
                    for col in 0..18 {
                        route.push(Pos(if row % 2 == 0 { 1 + col } else { 18 - col }, y));
                    }
                    if row < 4 {
                        route.push(Pos(if row % 2 == 0 { 18 } else { 1 }, y + 1));
                    }
                }
            } else {
                for col in 0..9 {
                    let x = 1 + col * 2;
                    for row in 0..10 {
                        route.push(Pos(x, if col % 2 == 0 { 1 + row } else { 10 - row }));
                    }
                    if col < 8 {
                        route.push(Pos(x + 1, if col % 2 == 0 { 10 } else { 1 }));
                    }
                }
            }
            let mut cells = [b'#'; 240];
            for p in &route {
                cells[idx(*p)] = b'.';
            }
            // Open shortcuts in early boards so the player can choose their route.
            if n < 6 {
                for y in 1..11 {
                    for x in 1..19 {
                        if (x + y + n as i32) % ((n + 3) as i32) != 0 {
                            cells[idx(Pos(x, y))] = b'.';
                        }
                    }
                }
            }
            let spacing = if n < 2 {
                9
            } else if n < 6 {
                5
            } else {
                2
            };
            for (i, p) in route.iter().enumerate().skip(1) {
                if i % spacing == 0 {
                    cells[idx(*p)] = b'A' + ((i / spacing + n) % 18) as u8;
                }
            }
            cells[idx(*route.last().unwrap())] = b'@';
            Level {
                name: name.to_string(),
                cells,
                start: route[0],
                solution: route.windows(2).map(|p| Dir::between(p[0], p[1])).collect(),
            }
        })
        .collect()
}

/// Read the author's uncompressed DOS 2.0 scene format. Comments and legacy
/// Czech text are ignored, so either DOS or Windows-1250 encoding is accepted.
pub fn parse_scn(bytes: &[u8]) -> Result<Vec<Level>, String> {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with(';'))
        .collect();
    let valid = |l: &str| {
        l.len() == 20
            && l.bytes()
                .all(|c| matches!(c, b'.' | b'$' | b'@' | b'#' | b'A'..=b'T'))
    };
    let mut levels = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if valid(lines[i]) {
            if i + 12 > lines.len() || !lines[i..i + 12].iter().all(|l| valid(l)) {
                return Err("Incomplete scene: expected twelve rows of twenty tiles".into());
            }
            let mut cells = [b'.'; 240];
            let mut start = None;
            let mut gates = 0;
            for (y, line) in lines[i..i + 12].iter().enumerate() {
                for (x, c) in line.bytes().enumerate() {
                    if c == b'$' {
                        if start.is_some() {
                            return Err("Scene has multiple locomotives".into());
                        }
                        start = Some(Pos(x as i32, y as i32));
                    }
                    if c == b'@' {
                        gates += 1;
                    }
                    cells[y * 20 + x] = if c == b'$' { b'.' } else { c };
                }
            }
            let start = start.ok_or("Scene has no locomotive")?;
            if gates != 1 {
                return Err("Scene must have exactly one exit".into());
            }
            let mut j = i;
            while j > 0 && lines[j - 1].starts_with('!') {
                j -= 1;
            }
            let name = if j > 0 && lines[j - 1].len() == 5 {
                lines[j - 1].to_string()
            } else {
                format!("SCENE {}", levels.len() + 1)
            };
            let solution = lines[j..i]
                .iter()
                .flat_map(|l| l.chars())
                .filter_map(|c| match c {
                    'U' => Some(Dir::Up),
                    'D' => Some(Dir::Down),
                    'L' => Some(Dir::Left),
                    'R' => Some(Dir::Right),
                    _ => None,
                })
                .collect();
            levels.push(Level {
                name,
                cells,
                start,
                solution,
            });
            i += 12;
        } else {
            i += 1;
        }
    }
    if levels.is_empty() {
        Err("No scenes found. Use the uncompressed VLAK.SCN file.".into())
    } else {
        Ok(levels)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ready,
    Playing,
    Paused,
    Crashed,
    Won,
}
pub struct Game {
    pub levels: Vec<Level>,
    pub level: usize,
    pub cells: [u8; 240],
    pub train: VecDeque<Pos>,
    pub cargo: Vec<u8>,
    pub dir: Dir,
    pub pending: Option<Dir>,
    pub remaining: usize,
    pub state: State,
    pub demo: bool,
    pub demo_step: usize,
}
impl Game {
    pub fn new(levels: Vec<Level>, level: usize) -> Self {
        let mut g = Self {
            levels,
            level: 0,
            cells: [b'.'; 240],
            train: VecDeque::new(),
            cargo: Vec::new(),
            dir: Dir::Right,
            pending: None,
            remaining: 0,
            state: State::Ready,
            demo: false,
            demo_step: 0,
        };
        g.load(level);
        g
    }
    pub fn load(&mut self, n: usize) {
        self.level = n.min(self.levels.len() - 1);
        let l = &self.levels[self.level];
        self.cells = l.cells;
        self.train = VecDeque::from([l.start]);
        self.cargo.clear();
        self.remaining = self
            .cells
            .iter()
            .filter(|c| matches!(c, b'A'..=b'T'))
            .count();
        self.dir = Dir::Right;
        self.pending = None;
        self.state = State::Ready;
        self.demo = false;
        self.demo_step = 0;
    }
    pub fn turn(&mut self, d: Dir) {
        if !matches!(self.state, State::Ready | State::Playing) || self.demo {
            return;
        }
        // Buffer one turn per movement tick; rapid keypresses cannot reverse
        // the train through its neck before a tick.
        if self.pending.is_some() {
            return;
        }
        if self.train.len() > 1 && d.step(self.train[0]) == self.train[1] {
            return;
        }
        self.pending = Some(d);
        self.state = State::Playing;
    }
    pub fn tick(&mut self) {
        if self.state != State::Playing {
            return;
        }
        if self.demo {
            if let Some(d) = self.levels[self.level].solution.get(self.demo_step) {
                self.pending = Some(*d);
                self.demo_step += 1;
            } else {
                self.state = State::Paused;
                return;
            }
        }
        if let Some(d) = self.pending.take() {
            self.dir = d;
        }
        let next = self.dir.step(self.train[0]);
        if !inside(next) {
            self.state = State::Crashed;
            return;
        }
        let tile = self.cells[idx(next)];
        // The original checks against every wagon, including the tail.
        if tile == b'#' || (tile == b'@' && self.remaining > 0) || self.train.contains(&next) {
            self.state = State::Crashed;
            return;
        }
        if matches!(tile, b'A'..=b'T') {
            self.cargo.push(tile);
            self.remaining -= 1;
            self.cells[idx(next)] = b'.';
        } else {
            self.train.pop_back();
        }
        self.train.push_front(next);
        if tile == b'@' {
            self.state = State::Won;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_bundled_solutions_win() {
        let levels = bundled();
        for i in 0..levels.len() {
            let mut g = Game::new(levels.clone(), i);
            g.demo = true;
            g.state = State::Playing;
            for _ in 0..300 {
                g.tick();
                if g.state != State::Playing {
                    break;
                }
            }
            assert_eq!(g.state, State::Won, "level {}", i + 1);
            assert_eq!(g.remaining, 0);
            assert_eq!(g.train.len(), g.cargo.len() + 1);
        }
    }
    #[test]
    fn all_fifty_original_solutions_win() {
        let levels = parse_scn(include_bytes!("../assets/original-levels.scn")).unwrap();
        assert_eq!(levels.len(), 50);
        for i in 0..levels.len() {
            let mut g = Game::new(levels.clone(), i);
            g.demo = true;
            g.state = State::Playing;
            for _ in 0..1000 {
                g.tick();
                if g.state != State::Playing {
                    break;
                }
            }
            assert_eq!(
                g.state,
                State::Won,
                "original level {} ({}) at step {}",
                i + 1,
                g.levels[i].name,
                g.demo_step
            );
            assert_eq!(g.remaining, 0);
        }
    }
    #[test]
    fn walls_crash_and_restart_resets() {
        let mut g = Game::new(bundled(), 0);
        g.turn(Dir::Up);
        g.tick();
        assert_eq!(g.state, State::Crashed);
        g.load(0);
        assert_eq!(g.state, State::Ready);
        assert_eq!(g.train.len(), 1);
    }
    #[test]
    fn locked_gate_crashes() {
        let mut g = Game::new(bundled(), 0);
        g.cells[idx(Pos(2, 1))] = b'@';
        g.turn(Dir::Right);
        g.tick();
        assert_eq!(g.state, State::Crashed);
    }
    #[test]
    fn cargo_grows_and_reverse_is_rejected() {
        let mut g = Game::new(bundled(), 0);
        g.cells[idx(Pos(2, 1))] = b'A';
        g.turn(Dir::Right);
        g.tick();
        assert_eq!(g.train.len(), 2);
        g.turn(Dir::Left);
        assert_eq!(g.pending, None);
    }
    #[test]
    fn import_scene() {
        let mut s = String::from("GOLEM\n!RRRS\n");
        for y in 0..12 {
            for x in 0..20 {
                s.push(if (x, y) == (1, 1) {
                    '$'
                } else if (x, y) == (18, 10) {
                    '@'
                } else {
                    '.'
                });
            }
            s.push('\n');
        }
        let l = parse_scn(s.as_bytes()).unwrap();
        assert_eq!(l[0].name, "GOLEM");
        assert_eq!(l[0].solution.len(), 3);
        assert_eq!(l[0].start, Pos(1, 1));
    }
    #[test]
    fn malformed_import_is_rejected() {
        assert!(parse_scn(b"GOLEM\n....................\n").is_err());
    }
}
