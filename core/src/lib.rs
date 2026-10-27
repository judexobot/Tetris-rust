//! Native (not 6502-emulated) NES Tetris logic + pixel renderer.
//! Tables were manually translated from the user-supplied TetrisNESDisasm archive.
//! 60 Hz tick; game graphics are original 8×8 assets extracted from the archive.
//! Audio is intentionally omitted.

use std::slice;

const W: usize = 10;
const H: usize = 20;
const NES_W: usize = 256;
const NES_H: usize = 240;

// Rust buttons are not NES's original bit positions. Keep these synced to the Android UI.
pub const LEFT: u32 = 1;
pub const RIGHT: u32 = 2;
pub const DOWN: u32 = 4;
pub const A: u32 = 8;
pub const B: u32 = 16;
pub const START: u32 = 32;
pub const SELECT: u32 = 64;
pub const UP: u32 = 128;

/// Order copied from constants.asm: T[0..4], J[4..8], Z[8..10], O[10],
/// S[11..13], L[13..17], I[17..19]. Each entry is [dy, dx] for four minos.
const SHAPES: [[(i8, i8); 4]; 19] = [
    [(0, -1), (0, 0), (0, 1), (-1, 0)],  // 0 T up
    [(-1, 0), (0, 0), (0, 1), (1, 0)],   // 1 T right
    [(0, -1), (0, 0), (0, 1), (1, 0)],   // 2 T down (spawn)
    [(-1, 0), (0, -1), (0, 0), (1, 0)],  // 3 T left
    [(-1, 0), (0, 0), (1, -1), (1, 0)],  // 4 J left
    [(-1, -1), (0, -1), (0, 0), (0, 1)], // 5 J up
    [(-1, 0), (-1, 1), (0, 0), (1, 0)],  // 6 J right
    [(0, -1), (0, 0), (0, 1), (1, 1)],   // 7 J down (spawn)
    [(0, -1), (0, 0), (1, 0), (1, 1)],   // 8 Z horiz (spawn)
    [(-1, 1), (0, 0), (0, 1), (1, 0)],   // 9 Z vert
    [(0, -1), (0, 0), (1, -1), (1, 0)],  // 10 O
    [(0, 0), (0, 1), (1, -1), (1, 0)],   // 11 S horiz (spawn)
    [(-1, 0), (0, 0), (0, 1), (1, 1)],   // 12 S vert
    [(-1, 0), (0, 0), (1, 0), (1, 1)],   // 13 L right
    [(0, -1), (0, 0), (0, 1), (1, -1)],  // 14 L down (spawn)
    [(-1, -1), (-1, 0), (0, 0), (1, 0)], // 15 L left
    [(-1, 1), (0, -1), (0, 0), (0, 1)],  // 16 L up
    [(-2, 0), (-1, 0), (0, 0), (1, 0)],  // 17 I vert
    [(0, -2), (0, -1), (0, 0), (0, 1)],  // 18 I horiz (spawn)
];

// Rotation table: [B = counterclockwise, A = clockwise].
const ROTATE: [[u8; 2]; 19] = [
    [3, 1],
    [0, 2],
    [1, 3],
    [2, 0],
    [7, 5],
    [4, 6],
    [5, 7],
    [6, 4],
    [9, 9],
    [8, 8],
    [10, 10],
    [12, 12],
    [11, 11],
    [16, 14],
    [13, 15],
    [14, 16],
    [15, 13],
    [18, 18],
    [17, 17],
];
const SPAWN: [u8; 7] = [2, 7, 8, 10, 11, 14, 18]; // T J Z O S L I
const KIND: [u8; 19] = [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 4, 5, 5, 6, 6, 6, 6, 7, 7];
const GRAVITY: [u8; 30] = [
    48, 43, 38, 33, 28, 23, 18, 13, 8, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    1,
];
const POINTS: [u32; 5] = [0, 40, 100, 300, 1200];

const GAME_GFX: &[u8; 16384] = include_bytes!("../assets/game.raw");
const MENU_GFX: &[u8; 16384] = include_bytes!("../assets/title_menu.raw");
const GAME_MAP: &[u8; 1024] = include_bytes!("../assets/game.map");
const TITLE_MAP: &[u8; 1024] = include_bytes!("../assets/title_screen.map");
const TYPE_MAP: &[u8; 1024] = include_bytes!("../assets/game_type_menu.map");
const LEVEL_MAP: &[u8; 1024] = include_bytes!("../assets/level_menu.map");
const NAME_ENTRY_MAP: &[u8; 1024] = include_bytes!("../assets/enter_high_score.map");
const END_A_GFX: &[u8; 16384] = include_bytes!("../assets/typeA_ending.raw");
const END_B_GFX: &[u8; 16384] = include_bytes!("../assets/typeB_ending.raw");
const END_B_MAP: &[u8; 1024] = include_bytes!("../assets/type_b_ending.map");
const END_B_9_MAP: &[u8; 1024] = include_bytes!("../assets/type_b_lvl9_ending.map");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    Title,
    GameType,
    LevelSelect,
    Playing,
    Paused,
    GameOver,
    Won,
    NameEntry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Record {
    score: u32,
    level: u8,
    name: [u8; 6],
}
impl Record {
    fn new(name: &[u8; 6], score: u32, level: u8) -> Self {
        let mut chars = [43; 6];
        for (i, c) in name.iter().enumerate() {
            if c.is_ascii_uppercase() {
                chars[i] = c - b'A' + 1;
            }
        }
        Self {
            score,
            level,
            name: chars,
        }
    }
    fn pack(self) -> u64 {
        let mut v = u64::from(self.score.min(999_999)) | (u64::from(self.level) << 20);
        for (i, c) in self.name.iter().enumerate() {
            v |= u64::from(*c) << (28 + i * 6);
        }
        v
    }
    fn unpack(v: u64) -> Self {
        let mut name = [0; 6];
        for (i, c) in name.iter_mut().enumerate() {
            *c = ((v >> (28 + i * 6)) & 63).min(43) as u8;
        }
        Self {
            score: ((v & 0xfffff) as u32).min(999_999),
            level: ((v >> 20) & 255) as u8,
            name,
        }
    }
}

pub struct Game {
    pub scene: Scene,
    pub field: [[u8; W]; H],
    pub kind: usize, // current orientation (0..19)
    pub next: usize, // next spawn orientation
    pub x: i8,
    pub y: i8,
    pub lines: u32,
    pub score: u32,
    pub level: u32,
    pub start_level: u32,
    pub game_type_b: bool,
    pub b_height: u8,
    pub show_next: bool,
    pub best_a: u32,
    pub best_b: u32,
    pub stats: [u32; 4], // singles / doubles / triples / tetrises
    pub piece_stats: [u32; 7],
    frame: u64,
    last_input: u32,
    das_counter: u8,
    gravity_timer: u8,
    soft_timer: u8, // NES autorepeatY: 0 = inactive, 1/2 = charging
    soft_score: u32,
    line_animation: u8,
    entry_delay: u8,
    lines_this_turn: u8,
    previous_piece: u8, // previous spawn orientation, not a piece index
    piece_counter: u8,
    rng: u16,
    selecting_height: bool,
    win_frame: u32,
    score_before_bonus: u32,
    records: [[Record; 3]; 2],
    record_row: usize,
    record_letter: usize,
    game_over_frame: u32,
}

impl Game {
    pub fn new() -> Self {
        // Cold boot seed; time spent in menus advances it once per frame.
        Self::with_seed(0x8889)
    }

    pub fn with_seed(seed: u32) -> Self {
        Self {
            scene: Scene::Title,
            field: [[0; W]; H],
            kind: 2,
            next: 2,
            x: 5,
            y: 0,
            lines: 0,
            score: 0,
            level: 0,
            start_level: 0,
            game_type_b: false,
            b_height: 0,
            show_next: true,
            best_a: 10_000,
            best_b: 2_000,
            stats: [0; 4],
            piece_stats: [0; 7],
            frame: 0,
            last_input: 0,
            das_counter: 0,
            gravity_timer: 0,
            soft_timer: 0,
            soft_score: 0,
            line_animation: 0,
            entry_delay: 0,
            lines_this_turn: 0,
            previous_piece: 0,
            piece_counter: 0,
            rng: if seed as u16 == 0 {
                0x8889
            } else {
                seed as u16
            },
            selecting_height: false,
            win_frame: 0,
            score_before_bonus: 0,
            records: [
                [
                    Record::new(b"HOWARD", 10_000, 9),
                    Record::new(b"OTASAN", 7_500, 5),
                    Record::new(b"LANCE ", 5_000, 0),
                ],
                [
                    Record::new(b"ALEX  ", 2_000, 9),
                    Record::new(b"TONY  ", 1_000, 5),
                    Record::new(b"NINTEN", 500, 0),
                ],
            ],
            record_row: 0,
            record_letter: 0,
            game_over_frame: 0,
        }
    }

    // generateNextPseudorandomNumber: shift the low byte first, with the
    // XOR of bit 1 of both bytes as carry, then shift the high byte.
    fn advance_rng(&mut self) {
        let lo = self.rng as u8;
        let hi = (self.rng >> 8) as u8;
        let new_lo = (lo >> 1) | (((lo ^ hi) & 2) << 6);
        let new_hi = (hi >> 1) | (lo << 7);
        self.rng = u16::from_le_bytes([new_lo, new_hi]);
    }

    fn pick_piece(&mut self) -> usize {
        // main.asm:pickRandomTetrimino. The fallback adds the previous
        // orientation ID (not the piece index) before reducing modulo seven.
        self.piece_counter = self.piece_counter.wrapping_add(1);
        let mut index = ((self.rng as u8).wrapping_add(self.piece_counter) & 7) as usize;
        if index == 7 || SPAWN[index] == self.previous_piece {
            self.advance_rng();
            index = ((self.rng as usize & 7) + usize::from(self.previous_piece)) % 7;
        }
        self.previous_piece = SPAWN[index];
        SPAWN[index] as usize
    }

    fn setup(&mut self) {
        self.field = [[0; W]; H];
        self.stats = [0; 4];
        self.piece_stats = [0; 7];
        self.level = self.start_level;
        self.score = 0;
        self.lines = if self.game_type_b { 25 } else { 0 };
        self.line_animation = 0;
        self.entry_delay = 0;
        self.win_frame = 0;
        self.soft_score = 0;
        self.gravity_timer = 0;
        self.soft_timer = 0;
        self.das_counter = 0;
        self.previous_piece = 0;
        self.kind = self.pick_piece();
        self.advance_rng();
        self.next = self.pick_piece();
        if self.game_type_b {
            self.generate_garbage();
        }
        self.scene = Scene::Playing;
        self.x = 5;
        self.y = 0;
        self.piece_stats[(KIND[self.kind] - 1) as usize] += 1;
        if !self.valid(self.x, self.y, self.kind) {
            self.game_over();
        }
    }

    fn generate_garbage(&mut self) {
        // Original Type-B layout: create 12 pseudo-random rows and then blank
        // the top according to height selection. The NES rngTable has 3/8
        // occupied cells using [blank,1,blank,2,3,3,blank,blank].
        for y in 8..H {
            for x in (0..W).rev() {
                const RNG_TABLE: [u8; 8] = [0, 1, 0, 3, 2, 2, 0, 0];
                self.advance_rng();
                self.field[y][x] = RNG_TABLE[(self.rng & 7) as usize];
            }
            // Rejection sampling is important: modulo would change the RNG stream.
            let hole = loop {
                self.advance_rng();
                let candidate = (self.rng & 15) as usize;
                if candidate < W {
                    break candidate;
                }
            };
            self.field[y][hole] = 0;
            self.advance_rng(); // one NMI while building each row
        }
        let last_blank = [200, 170, 150, 120, 100, 80][usize::from(self.b_height.min(5))];
        for cell in 0..=last_blank.min(199) {
            self.field[cell / W][cell % W] = 0;
        }
    }

    fn spawn(&mut self) {
        self.kind = self.next;
        self.next = self.pick_piece();
        self.x = 5;
        self.y = 0;
        self.gravity_timer = 0;
        self.soft_timer = 0;
        self.soft_score = 0;
        self.piece_stats[(KIND[self.kind] - 1) as usize] += 1;
        if !self.valid(self.x, self.y, self.kind) {
            self.game_over();
        }
    }

    fn valid(&self, x: i8, y: i8, kind: usize) -> bool {
        SHAPES[kind].iter().all(|&(dy, dx)| {
            let row = y as i16 + dy as i16;
            let col = x as i16 + dx as i16;
            col >= 0
                && col < W as i16
                && row >= -2
                && row < H as i16
                && (row < 0 || self.field[row as usize][col as usize] == 0)
        })
    }

    fn shift(&mut self, dx: i8) -> bool {
        let x = self.x + dx;
        if self.valid(x, self.y, self.kind) {
            self.x = x;
            true
        } else {
            false
        }
    }

    fn rotate(&mut self, clockwise: bool) -> bool {
        let new_kind = ROTATE[self.kind][usize::from(clockwise)] as usize;
        // No wall kicks, as in the original NES game.
        if self.valid(self.x, self.y, new_kind) {
            self.kind = new_kind;
            true
        } else {
            false
        }
    }

    fn down_step(&mut self) -> bool {
        if self.valid(self.x, self.y + 1, self.kind) {
            self.y += 1;
            true
        } else {
            self.lock();
            false
        }
    }

    fn lock(&mut self) {
        let mut overflow = false;
        for &(dy, dx) in SHAPES[self.kind].iter() {
            let row = self.y + dy;
            let col = self.x + dx;
            if row < 0 {
                overflow = true;
            } else if row < H as i8 && col >= 0 && col < W as i8 {
                self.field[row as usize][col as usize] = KIND[self.kind];
            }
        }
        if overflow {
            self.game_over();
            return;
        }
        self.lines_this_turn = self
            .field
            .iter()
            .filter(|row| row.iter().all(|&v| v != 0))
            .count() as u8;
        if self.lines_this_turn > 0 {
            // Five center-outward pairs, one pair each four video frames.
            self.line_animation = 20;
        } else {
            self.finish_lock();
        }
    }

    fn clear_lines(&mut self) -> u8 {
        let mut write = H as isize - 1;
        let mut removed = 0;
        for read in (0..H).rev() {
            if self.field[read].iter().all(|&b| b != 0) {
                removed += 1;
            } else {
                self.field[write as usize] = self.field[read];
                write -= 1;
            }
        }
        while write >= 0 {
            self.field[write as usize] = [0; W];
            write -= 1;
        }
        removed
    }

    fn game_over(&mut self) {
        self.scene = Scene::GameOver;
        self.game_over_frame = 0;
    }

    fn finish_lock(&mut self) {
        let cleared = self.lines_this_turn as usize;
        if cleared > 0 {
            self.clear_lines();
            self.stats[cleared.saturating_sub(1).min(3)] += 1;
        }
        if self.game_type_b {
            self.lines = self.lines.saturating_sub(self.lines_this_turn as u32);
        } else {
            // The comparison in updateLinesAndStatistics shifts packed BCD
            // right four bits and compares that byte as binary. This matters
            // for starts above 9 and on every subsequent transition.
            for _ in 0..cleared {
                self.lines += 1;
                if self.lines % 10 == 0 {
                    let packed_tens = ((self.lines / 100) << 4) | ((self.lines % 100) / 10);
                    if packed_tens > self.level {
                        self.level += 1;
                    }
                }
            }
        }
        // The level is incremented before the line-clear score is added.
        let gain = POINTS[cleared.min(4)] * (self.level + 1);
        let soft = if self.soft_score >= 2 {
            self.soft_score - 1
        } else {
            0
        };
        self.score = self.score.saturating_add(gain + soft).min(999_999);
        if self.game_type_b && self.lines == 0 {
            // endingAnimationB uses the displayed units digit of the level.
            self.score_before_bonus = self.score;
            self.win_frame = 0;
            let bonus = (self.start_level % 10 + u32::from(self.b_height)) * 1_000;
            self.score = self.score.saturating_add(bonus).min(999_999);
            self.scene = Scene::Won;
        }
        if self.game_type_b {
            self.best_b = self.best_b.max(self.score);
        } else {
            self.best_a = self.best_a.max(self.score);
        }
        self.lines_this_turn = 0;
        if self.scene == Scene::Playing {
            // Entry delay varies with lock height. The original's PPU row
            // updates also influence its exact timing; see fidelity notes.
            self.entry_delay = 10 + 2 * (self.y.max(0) as u8 / 4).min(4);
        }
    }

    /// Advance by one 60 Hz frame. Buttons are a bitmask and may be held across frames.
    pub fn tick(&mut self, input: u32) {
        self.frame = self.frame.wrapping_add(1);
        self.advance_rng(); // NES NMI advances the RNG every video frame.
        let pressed = input & !self.last_input;
        self.last_input = input;
        match self.scene {
            Scene::Title => {
                if pressed & (START | A) != 0 {
                    self.scene = Scene::GameType;
                }
            }
            Scene::GameType => {
                if pressed & LEFT != 0 {
                    self.game_type_b = false;
                }
                if pressed & RIGHT != 0 {
                    self.game_type_b = true;
                }
                if pressed & START != 0 {
                    self.selecting_height = false;
                    self.scene = Scene::LevelSelect;
                } else if pressed & B != 0 {
                    self.scene = Scene::Title;
                }
            }
            Scene::LevelSelect => {
                if pressed & A != 0 && pressed & START == 0 && self.game_type_b {
                    self.selecting_height = !self.selecting_height;
                }
                if self.selecting_height && self.game_type_b {
                    if pressed & LEFT != 0 {
                        self.b_height = self.b_height.saturating_sub(1);
                    }
                    if pressed & RIGHT != 0 {
                        self.b_height = (self.b_height + 1).min(5);
                    }
                    if pressed & UP != 0 && self.b_height >= 3 {
                        self.b_height -= 3;
                    }
                    if pressed & DOWN != 0 && self.b_height < 3 {
                        self.b_height += 3;
                    }
                } else {
                    if pressed & LEFT != 0 {
                        self.start_level = self.start_level.saturating_sub(1);
                    }
                    if pressed & RIGHT != 0 {
                        self.start_level = (self.start_level + 1).min(9);
                    }
                    if pressed & UP != 0 && self.start_level >= 5 {
                        self.start_level -= 5;
                    }
                    if pressed & DOWN != 0 && self.start_level < 5 {
                        self.start_level += 5;
                    }
                }
                if pressed & START != 0 {
                    // NES hidden levels 10-19: hold A while pressing START.
                    if input & A != 0 {
                        self.start_level += 10;
                    }
                    self.setup();
                } else if pressed & B != 0 {
                    self.scene = Scene::GameType;
                } else {
                    // Level menu selects two garbage holes each frame even in
                    // single-player mode, using 0..9 rejection sampling.
                    for _ in 0..2 {
                        loop {
                            self.advance_rng();
                            if self.rng & 15 < 10 {
                                break;
                            }
                        }
                    }
                }
            }
            Scene::Paused => {
                if pressed & START != 0 {
                    self.scene = Scene::Playing;
                }
                if pressed & SELECT != 0 {
                    self.scene = Scene::Title;
                }
            }
            Scene::GameOver => {
                self.game_over_frame = self.game_over_frame.saturating_add(1);
                if pressed & START != 0 && self.game_over_frame >= 144 {
                    self.enter_records_or_menu();
                }
            }
            Scene::Won => {
                self.win_frame = self.win_frame.saturating_add(1);
                if pressed & START != 0 && self.win_frame >= self.win_duration() {
                    self.enter_records_or_menu();
                }
            }
            Scene::NameEntry => {
                if pressed & START != 0 {
                    self.start_level %= 10;
                    self.selecting_height = false;
                    self.scene = Scene::LevelSelect;
                } else {
                    if pressed & (A | RIGHT) != 0 {
                        self.record_letter = (self.record_letter + 1) % 6;
                    }
                    if pressed & (B | LEFT) != 0 {
                        self.record_letter = (self.record_letter + 5) % 6;
                    }
                    if self.frame % 8 == 0 {
                        let letter = &mut self.records[usize::from(self.game_type_b)]
                            [self.record_row]
                            .name[self.record_letter];
                        if input & DOWN != 0 {
                            *letter = (*letter + 43) % 44;
                        }
                        if input & UP != 0 {
                            *letter = (*letter + 1) % 44;
                        }
                    }
                }
            }
            Scene::Playing => {
                if pressed & START != 0 {
                    self.scene = Scene::Paused;
                    return;
                }
                if pressed & SELECT != 0 {
                    self.show_next = !self.show_next;
                }
                if self.line_animation > 0 {
                    self.line_animation -= 1;
                    if self.line_animation == 0 {
                        self.finish_lock();
                    }
                    return;
                }
                if self.entry_delay > 0 {
                    self.entry_delay -= 1;
                    if self.entry_delay == 0 {
                        self.spawn();
                    }
                    return;
                }
                // shift_tetrimino: a failed move charges DAS, and the charge
                // survives piece entry. Right has priority if both are held.
                if input & DOWN == 0 && input & (LEFT | RIGHT) != 0 {
                    let new_direction = pressed & (LEFT | RIGHT) != 0;
                    if new_direction {
                        self.das_counter = 0;
                    } else {
                        self.das_counter = self.das_counter.saturating_add(1);
                    }
                    if new_direction || self.das_counter >= 16 {
                        if !new_direction {
                            self.das_counter = 10;
                        }
                        let dir = if input & RIGHT != 0 { 1 } else { -1 };
                        if !self.shift(dir) {
                            self.das_counter = 16;
                        }
                    }
                }
                if pressed & A != 0 {
                    self.rotate(true);
                } else if pressed & B != 0 {
                    self.rotate(false);
                }
                self.gravity_timer = self.gravity_timer.saturating_add(1);
                let every = GRAVITY[self.level.min(29) as usize];
                if self.soft_timer == 0 {
                    if pressed & (UP | DOWN | LEFT | RIGHT) == DOWN && input & (LEFT | RIGHT) == 0 {
                        self.soft_timer = 1;
                    }
                } else if input & (UP | DOWN | LEFT | RIGHT) != DOWN {
                    self.soft_timer = 0;
                    self.soft_score = 0;
                } else {
                    self.soft_timer += 1;
                    if self.soft_timer >= 3 {
                        self.soft_timer = 1;
                        self.gravity_timer = 0;
                        self.soft_score = self.soft_score.saturating_add(1);
                        self.down_step();
                        return;
                    }
                }
                if self.gravity_timer >= every {
                    self.gravity_timer = 0;
                    self.down_step();
                }
            }
        }
    }

    /// Pause gameplay when Android loses focus; resuming requires START.
    pub fn pause_for_focus_loss(&mut self) {
        if self.scene == Scene::Playing {
            self.scene = Scene::Paused;
        }
        self.last_input = 0;
    }

    fn enter_records_or_menu(&mut self) {
        let records = &mut self.records[usize::from(self.game_type_b)];
        if let Some(row) = records.iter().position(|r| self.score >= r.score) {
            for i in (row + 1..3).rev() {
                records[i] = records[i - 1];
            }
            records[row] = Record {
                score: self.score,
                level: self.level.min(255) as u8,
                name: [0; 6],
            };
            self.record_row = row;
            self.record_letter = 0;
            self.scene = Scene::NameEntry;
        } else {
            self.start_level %= 10;
            self.selecting_height = false;
            self.scene = Scene::LevelSelect;
        }
    }

    fn win_duration(&self) -> u32 {
        let level = self.start_level % 10;
        let height = u32::from(self.b_height);
        128 + 64
            + level * 20
            + if level > 0 { 64 } else { 0 }
            + height * 20
            + if height > 0 { 64 } else { 0 }
    }

    // The original counts each 100-point bonus at two-frame intervals.
    fn win_display(&self) -> (u32, u32, u32) {
        let level = (self.start_level % 10) * 1000;
        let height = u32::from(self.b_height) * 1000;
        let mut t = self.win_frame.saturating_sub(128 + 64);
        let level_paid = (t / 2).min(level / 100) * 100;
        t = t.saturating_sub((level / 100) * 2);
        if level > 0 {
            t = t.saturating_sub(64);
        }
        let height_paid = (t / 2).min(height / 100) * 100;
        (
            level - level_paid,
            height - height_paid,
            self.score_before_bonus
                .saturating_add(level_paid + height_paid)
                .min(999_999),
        )
    }

    /// 256x240 ARGB pixel framebuffer, scaled externally by Android.
    pub fn render(&self, pixels: &mut [u32]) {
        if pixels.len() < NES_W * NES_H {
            return;
        }
        pixels[..NES_W * NES_H].fill(0xFF080817);
        let game_palette = game_palette(self.level);
        let ending_palette = ending_palette();
        let (gfx, map, pal): (&[u8; 16384], &[u8; 1024], &[[u32; 4]; 4]) = match self.scene {
            Scene::Title => (MENU_GFX, TITLE_MAP, &MENU_PALETTE),
            Scene::GameType => (MENU_GFX, TYPE_MAP, &MENU_PALETTE),
            Scene::LevelSelect => (MENU_GFX, LEVEL_MAP, &MENU_PALETTE),
            Scene::NameEntry => (MENU_GFX, NAME_ENTRY_MAP, &MENU_PALETTE),
            Scene::Won if self.game_type_b && self.start_level % 10 == 9 => {
                (END_B_GFX, END_B_9_MAP, &ending_palette)
            }
            Scene::Won if self.game_type_b => (END_A_GFX, END_B_MAP, &ending_palette),
            _ => (GAME_GFX, GAME_MAP, &game_palette),
        };
        for row in 0..30 {
            for col in 0..32 {
                let tile = map[row * 32 + col];
                let attr = map[960 + (row / 4) * 8 + (col / 4)];
                let shift = ((row % 4) / 2 * 4 + (col % 4) / 2 * 2) as u32;
                let pal_id = (attr >> shift) & 3;
                let bank = if self.scene == Scene::Won
                    && self.game_type_b
                    && matches!(self.start_level % 10, 2 | 6)
                    && tile < 128
                {
                    GAME_GFX
                } else {
                    gfx
                };
                draw_tile(
                    pixels,
                    (col * 8) as i32,
                    (row * 8) as i32,
                    bank,
                    tile,
                    pal[pal_id as usize],
                );
            }
        }
        if self.scene == Scene::Won && self.game_type_b {
            let (level_bonus, height_bonus, total) = self.win_display();
            let color = ending_palette[1];
            draw_digits(pixels, 112, 32, self.score_before_bonus, 6, gfx, color);
            draw_digits(pixels, 120, 40, level_bonus, 5, gfx, color);
            draw_digits(pixels, 120, 48, height_bonus, 5, gfx, color);
            draw_digits(pixels, 112, 72, total, 6, gfx, color);
            return;
        }
        match self.scene {
            Scene::Title => {}
            Scene::GameType => {
                // Menu selection box, on top of original nametable.
                let x = if self.game_type_b { 159 } else { 63 };
                outline(pixels, x, 63, 48, 16, 0xFFFFFFFF);
            }
            Scene::LevelSelect => {
                let col = (self.start_level % 5) as i32;
                let row = ((self.start_level % 10) / 5) as i32;
                if !self.selecting_height || self.frame % 4 != 0 {
                    outline(pixels, 52 + col * 16, 83 + row * 16, 16, 16, 0xFFFFFFFF);
                }
                if self.game_type_b {
                    if self.selecting_height || self.frame % 4 != 0 {
                        outline(
                            pixels,
                            156 + (self.b_height % 3) as i32 * 16,
                            83 + (self.b_height / 3) as i32 * 16,
                            16,
                            16,
                            0xFFFFFFFF,
                        );
                    }
                } else {
                    shade_rect(pixels, 101, 22, 58, 15, 0xFF090914);
                    draw_text(pixels, 104, 24, "A-TYPE", 0xFFFFFFFF);
                }
                self.draw_records(pixels);
            }
            Scene::NameEntry => {
                self.draw_records(pixels);
                draw_tile(
                    pixels,
                    104,
                    24,
                    MENU_GFX,
                    if self.game_type_b { 0x0b } else { 0x0a },
                    MENU_PALETTE[0],
                );
                if self.frame % 4 != 0 {
                    outline(
                        pixels,
                        72 + self.record_letter as i32 * 8,
                        159 + self.record_row as i32 * 16,
                        8,
                        9,
                        0xFFFFFFFF,
                    );
                }
            }
            _ => {
                draw_text(
                    pixels,
                    24,
                    32,
                    if self.game_type_b { "B" } else { "A" },
                    0xFFFFFFFF,
                );
                // Aligned to the NES PPU screen layout (game field at x=96, y=48).
                for row in 0..H {
                    for col in 0..W {
                        let id = self.field[row][col];
                        if id != 0 {
                            let full = self.field[row].iter().all(|&v| v != 0);
                            let erased_pairs = (20 - self.line_animation as usize) / 4;
                            let erased = self.line_animation > 0
                                && full
                                && col >= 5 - erased_pairs
                                && col < 5 + erased_pairs;
                            if !erased {
                                draw_mino(
                                    pixels,
                                    (96 + col * 8) as i32,
                                    (48 + row * 8) as i32,
                                    id,
                                    game_palette[2],
                                );
                            }
                        }
                    }
                }
                if matches!(self.scene, Scene::Playing | Scene::Paused) {
                    if self.line_animation == 0 && self.entry_delay == 0 {
                        for &(dy, dx) in &SHAPES[self.kind] {
                            let row = self.y + dy;
                            let col = self.x + dx;
                            if row >= 0 && row < H as i8 && col >= 0 && col < W as i8 {
                                draw_mino(
                                    pixels,
                                    96 + col as i32 * 8,
                                    48 + row as i32 * 8,
                                    KIND[self.kind],
                                    game_palette[2],
                                );
                            }
                        }
                    }
                }
                // Dynamic status at the original nametable text positions.
                draw_text(
                    pixels,
                    152,
                    24,
                    &format!("{:03}", self.lines.min(999)),
                    0xFFFFC94D,
                );
                draw_text(pixels, 192, 64, &format!("{:06}", self.score), 0xFFFFC94D);
                draw_text(
                    pixels,
                    208,
                    168,
                    &format!("{:02}", self.level.min(99)),
                    0xFFFFC94D,
                );
                draw_text(
                    pixels,
                    192,
                    40,
                    &format!(
                        "{:06}",
                        if self.game_type_b {
                            self.best_b
                        } else {
                            self.best_a
                        }
                    ),
                    0xFFFFFFFF,
                );
                for (i, count) in self.piece_stats.iter().enumerate() {
                    draw_text(
                        pixels,
                        48,
                        96 + i as i32 * 16,
                        &format!("{:03}", count.min(&999)),
                        0xFFFFFFFF,
                    );
                }
                if self.show_next && matches!(self.scene, Scene::Playing | Scene::Paused) {
                    for &(dy, dx) in &SHAPES[self.next] {
                        draw_mino(
                            pixels,
                            204 + dx as i32 * 8,
                            122 + dy as i32 * 8,
                            KIND[self.next],
                            game_palette[2],
                        );
                    }
                }
                if self.scene == Scene::GameOver {
                    let rows = (self.game_over_frame / 4).saturating_sub(16).min(H as u32);
                    for row in 0..rows {
                        for col in 0..W {
                            draw_tile(
                                pixels,
                                96 + col as i32 * 8,
                                48 + row as i32 * 8,
                                GAME_GFX,
                                0x4f,
                                game_palette[2],
                            );
                        }
                    }
                }
                match self.scene {
                    Scene::Paused => {
                        shade_rect(pixels, 98, 107, 78, 23, 0xFF090B18);
                        draw_text(pixels, 116, 114, "PAUSE", 0xFFFFFFFF);
                    }
                    Scene::Won => {
                        shade_rect(pixels, 90, 104, 95, 26, 0xFF090B18);
                        draw_text(pixels, 112, 113, "CLEAR!", 0xFFFFFFFF);
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_records(&self, pixels: &mut [u32]) {
        const CHARS: [u8; 44] = [
            0x24, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0, 1, 2,
            3, 4, 5, 6, 7, 8, 9, 0x25, 0x4f, 0x5e, 0x5f, 0x6e, 0x6f, 0xff,
        ];
        for (row, record) in self.records[usize::from(self.game_type_b)]
            .iter()
            .enumerate()
        {
            let y = 160 + row as i32 * 16;
            for (i, c) in record.name.iter().enumerate() {
                draw_tile(
                    pixels,
                    72 + i as i32 * 8,
                    y,
                    MENU_GFX,
                    CHARS[*c as usize],
                    MENU_PALETTE[0],
                );
            }
            draw_digits(pixels, 128, y, record.score, 6, MENU_GFX, MENU_PALETTE[0]);
            draw_digits(
                pixels,
                184,
                y,
                u32::from(record.level).min(99),
                2,
                MENU_GFX,
                MENU_PALETTE[0],
            );
        }
    }
}

// RGB approximation of the NTSC NES palette. Palette indices are taken from
// game_palette, menu_palette and colorTable in the reference disassembly.
const NES_RGB: [u32; 64] = [
    0xff666666, 0xff002a88, 0xff1412a7, 0xff3b00a4, 0xff5c007e, 0xff6e0040, 0xff6c0600, 0xff561d00,
    0xff333500, 0xff0b4800, 0xff005200, 0xff004f08, 0xff00404d, 0xff000000, 0xff000000, 0xff000000,
    0xffadadad, 0xff155fd9, 0xff4240ff, 0xff7527fe, 0xffa01acc, 0xffb71e7b, 0xffb53120, 0xff994e00,
    0xff6b6d00, 0xff388700, 0xff0c9300, 0xff008f32, 0xff007c8d, 0xff000000, 0xff000000, 0xff000000,
    0xfffffeff, 0xff64b0ff, 0xff9290ff, 0xffc676ff, 0xfff36aff, 0xfffe6ecc, 0xfffe8170, 0xffea9e22,
    0xffbcbe00, 0xff88d800, 0xff5ce430, 0xff45e082, 0xff48cdde, 0xff4f4f4f, 0xff000000, 0xff000000,
    0xfffffeff, 0xffc0dfff, 0xffd3d2ff, 0xffe8c8ff, 0xfffbc2ff, 0xfffec4ea, 0xfffeccc5, 0xfff7d8a5,
    0xffe4e594, 0xffcfef96, 0xffbdf4ab, 0xffb3f3cc, 0xffb5ebf2, 0xffb8b8b8, 0xff000000, 0xff000000,
];
fn game_palette(level: u32) -> [[u32; 4]; 4] {
    const LEVEL_COLORS: [[usize; 2]; 10] = [
        [0x21, 0x12],
        [0x29, 0x1a],
        [0x24, 0x14],
        [0x2a, 0x12],
        [0x2b, 0x15],
        [0x22, 0x2b],
        [0x00, 0x16],
        [0x05, 0x13],
        [0x16, 0x12],
        [0x27, 0x16],
    ];
    let c = LEVEL_COLORS[(level % 10) as usize];
    [
        [NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[0x12], NES_RGB[0x16]],
        [NES_RGB[0x0f], NES_RGB[0x20], NES_RGB[0x12], NES_RGB[0x18]],
        [NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[c[0]], NES_RGB[c[1]]],
        [NES_RGB[0x0f], NES_RGB[0x3c], NES_RGB[0x00], NES_RGB[0x30]],
    ]
}
fn ending_palette() -> [[u32; 4]; 4] {
    [
        [NES_RGB[0x12], NES_RGB[0x0f], NES_RGB[0x29], NES_RGB[0x37]],
        [NES_RGB[0x12], NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[0x27]],
        [NES_RGB[0x12], NES_RGB[0x0f], NES_RGB[0x17], NES_RGB[0x27]],
        [NES_RGB[0x12], NES_RGB[0x0f], NES_RGB[0x15], NES_RGB[0x37]],
    ]
}
const MENU_PALETTE: [[u32; 4]; 4] = [
    [NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[0x38], NES_RGB[0x00]],
    [NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[0x16], NES_RGB[0x00]],
    [NES_RGB[0x0f], NES_RGB[0x30], NES_RGB[0x21], NES_RGB[0x00]],
    [NES_RGB[0x0f], NES_RGB[0x16], NES_RGB[0x2a], NES_RGB[0x28]],
];

fn draw_tile(
    pixels: &mut [u32],
    left: i32,
    top: i32,
    tiles: &[u8; 16384],
    id: u8,
    palette: [u32; 4],
) {
    let offset = id as usize * 64;
    for y in 0..8i32 {
        let dy = top + y;
        if !(0..NES_H as i32).contains(&dy) {
            continue;
        }
        for x in 0..8i32 {
            let dx = left + x;
            if !(0..NES_W as i32).contains(&dx) {
                continue;
            }
            let shade = tiles[offset + (y * 8 + x) as usize] as usize;
            pixels[dy as usize * NES_W + dx as usize] = palette[shade];
        }
    }
}
fn draw_mino(pixels: &mut [u32], x: i32, y: i32, kind: u8, pal: [u32; 4]) {
    let tile = match kind {
        1 | 4 | 7 => 0x7b,
        2 | 5 => 0x7d,
        _ => 0x7c,
    };
    draw_tile(pixels, x, y, GAME_GFX, tile, pal);
}
fn draw_digits(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    value: u32,
    width: usize,
    gfx: &[u8; 16384],
    palette: [u32; 4],
) {
    for (n, byte) in format!("{value:0width$}").bytes().enumerate() {
        draw_tile(pixels, x + n as i32 * 8, y, gfx, byte - b'0', palette);
    }
}
fn draw_text(pixels: &mut [u32], x: i32, y: i32, text: &str, color: u32) {
    // 0..9 are the numbers in original game_tileset; letters A..Z at 0x0a..0x23.
    for (n, ch) in text.chars().enumerate() {
        let tile = match ch {
            '0'..='9' => ch as u8 - b'0',
            'A'..='Z' => 0x0a + ch as u8 - b'A',
            '-' => 0x2c,
            '!' => 0x24,
            _ => 0xef,
        };
        draw_tile(
            pixels,
            x + n as i32 * 8,
            y,
            GAME_GFX,
            tile,
            [0xFF000000, color, color, color],
        );
    }
}
fn outline(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    for dy in 0..h {
        for dx in 0..w {
            if dy == 0 || dy == h - 1 || dx == 0 || dx == w - 1 {
                put(pixels, x + dx, y + dy, color);
            }
        }
    }
}
fn shade_rect(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, color: u32) {
    for dy in 0..h {
        for dx in 0..w {
            put(pixels, x + dx, y + dy, color);
        }
    }
}
fn put(pixels: &mut [u32], x: i32, y: i32, color: u32) {
    if x >= 0 && x < NES_W as i32 && y >= 0 && y < NES_H as i32 {
        pixels[y as usize * NES_W + x as usize] = color;
    }
}

// Native C ABI - the only dependency of the NDK C++ JNI shim.
#[no_mangle]
pub extern "C" fn tr_create() -> *mut Game {
    Box::into_raw(Box::new(Game::new()))
}

/// # Safety
/// `handle` must be a valid pointer previously returned by `tr_create`, and
/// must be released exactly once.
#[no_mangle]
pub unsafe extern "C" fn tr_destroy(handle: *mut Game) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// # Safety
/// `handle` must point to a live `Game`.
#[no_mangle]
pub unsafe extern "C" fn tr_tick(handle: *mut Game, buttons: u32) {
    if let Some(game) = handle.as_mut() {
        game.tick(buttons);
    }
}

/// # Safety
/// `handle` must point to a live `Game` or be null.
#[no_mangle]
pub unsafe extern "C" fn tr_pause_for_focus_loss(handle: *mut Game) {
    if let Some(game) = handle.as_mut() {
        game.pause_for_focus_loss();
    }
}

/// # Safety
/// `handle` must point to a live `Game`, and `out` must point to at least
/// 256×240 valid u32 pixels (ARGB_8888); `pixels_count` is checked.
#[no_mangle]
pub unsafe extern "C" fn tr_render(handle: *mut Game, out: *mut u32, pixels_count: usize) {
    if out.is_null() || pixels_count < NES_W * NES_H {
        return;
    }
    if let Some(game) = handle.as_ref() {
        game.render(slice::from_raw_parts_mut(out, NES_W * NES_H));
    }
}

/// Current score for persistence on the Android side.
#[no_mangle]
pub unsafe extern "C" fn tr_score(handle: *const Game) -> u32 {
    handle.as_ref().map_or(0, |g| g.score)
}

/// Scene number: title=0, gameType=1, levelSelect=2, playing=3,
/// paused=4, gameOver=5, won=6, name entry=7.
#[no_mangle]
pub unsafe extern "C" fn tr_scene(handle: *const Game) -> u32 {
    handle.as_ref().map_or(0, |g| match g.scene {
        Scene::Title => 0,
        Scene::GameType => 1,
        Scene::LevelSelect => 2,
        Scene::Playing => 3,
        Scene::Paused => 4,
        Scene::GameOver => 5,
        Scene::Won => 6,
        Scene::NameEntry => 7,
    })
}
#[no_mangle]
pub unsafe extern "C" fn tr_game_type_b(handle: *const Game) -> bool {
    handle.as_ref().is_some_and(|g| g.game_type_b)
}
#[no_mangle]
pub unsafe extern "C" fn tr_best(handle: *const Game, b: bool) -> u32 {
    handle
        .as_ref()
        .map_or(0, |g| if b { g.best_b } else { g.best_a })
}
#[no_mangle]
pub unsafe extern "C" fn tr_set_best(handle: *mut Game, a: u32, b: u32) {
    if let Some(g) = handle.as_mut() {
        g.best_a = a.min(999_999).max(g.records[0][0].score);
        g.best_b = b.min(999_999).max(g.records[1][0].score);
    }
}

/// # Safety
/// `handle` must point to a live `Game` or be null. Out-of-range rows are ignored.
#[no_mangle]
pub unsafe extern "C" fn tr_record(handle: *const Game, b: bool, row: u32) -> u64 {
    handle
        .as_ref()
        .and_then(|g| g.records[usize::from(b)].get(row as usize))
        .map_or(0, |r| r.pack())
}

/// # Safety
/// `handle` must point to a live `Game` or be null. Out-of-range rows are ignored.
#[no_mangle]
pub unsafe extern "C" fn tr_set_record(handle: *mut Game, b: bool, row: u32, value: u64) {
    if let Some(g) = handle.as_mut() {
        if let Some(record) = g.records[usize::from(b)].get_mut(row as usize) {
            *record = Record::unpack(value);
            if row == 0 {
                if b {
                    g.best_b = g.best_b.max(record.score);
                } else {
                    g.best_a = g.best_a.max(record.score);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::with_seed(42);
        g.setup();
        g
    }
    #[test]
    fn all_piece_shapes_contain_four_unique_squares() {
        for s in SHAPES {
            for i in 0..4 {
                for j in i + 1..4 {
                    assert_ne!(s[i], s[j]);
                }
            }
        }
    }
    #[test]
    fn original_tables_are_intact() {
        assert_eq!(SPAWN, [2, 7, 8, 10, 11, 14, 18]);
        assert_eq!(GRAVITY[0], 48);
        assert_eq!(GRAVITY[9], 6);
        assert_eq!(GRAVITY[29], 1);
        assert_eq!(ROTATE[2], [1, 3]);
    }
    #[test]
    fn rotation_never_kicks_outside_board() {
        let mut g = game();
        g.kind = 17;
        g.x = 1;
        g.y = 5;
        assert!(!g.rotate(false));
        assert_eq!(g.kind, 17);
    }
    #[test]
    fn clears_multiple_rows_and_shifts() {
        let mut g = game();
        g.field[19] = [1; 10];
        g.field[18] = [2; 10];
        g.field[17][5] = 7;
        assert_eq!(g.clear_lines(), 2);
        assert_eq!(g.field[19][5], 7);
        assert_eq!(g.field[18], [0; 10]);
    }
    #[test]
    fn nes_scoring_and_level_transition() {
        let mut g = game();
        g.lines_this_turn = 4;
        g.finish_lock();
        assert_eq!(g.score, 1200);
        assert_eq!(g.lines, 4);
        g.lines = 8;
        g.lines_this_turn = 2;
        g.finish_lock();
        assert_eq!(g.level, 1);
    }
    #[test]
    fn game_over_when_occupied_spawn() {
        let mut g = game();
        g.field[0] = [1; 10];
        g.field[1] = [1; 10];
        g.spawn();
        assert_eq!(g.scene, Scene::GameOver);
    }
    #[test]
    fn renderer_writes_frame() {
        let g = game();
        let mut pixels = vec![0; 256 * 240];
        g.render(&mut pixels);
        assert!(pixels.iter().any(|&x| x != 0xFF080817));
    }
    #[test]
    fn correct_type_b_height_garbage() {
        let mut g = Game::with_seed(123);
        g.game_type_b = true;
        g.b_height = 0;
        g.setup();
        assert!(g.field.iter().all(|row| row.iter().all(|&x| x == 0)));
        g.b_height = 5;
        g.setup();
        assert!(g.field[0..8].iter().all(|row| row.iter().all(|&x| x == 0)));
        assert!(g.field[8..20].iter().any(|row| row.iter().any(|&x| x != 0)));
        assert!(g.field.iter().all(|row| row.iter().any(|&x| x == 0)));
    }
    #[test]
    fn finished_rows_flash_before_collapse() {
        let mut g = game();
        g.field[19] = [1; 10];
        g.lines_this_turn = 1;
        g.line_animation = 1;
        g.tick(0);
        assert_eq!(g.field[19], [0; 10]);
        assert_eq!(g.stats[0], 1);
    }
    #[test]
    fn type_b_awards_level_height_bonus_on_goal() {
        let mut g = game();
        g.game_type_b = true;
        g.start_level = 9;
        g.b_height = 5;
        g.setup();
        g.lines = 1;
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!(g.scene, Scene::Won);
        assert_eq!(g.score, 14_400);
    }
    #[test]
    fn best_scores_independent_of_game_modes() {
        let mut g = game();
        g.score = 12_000;
        g.lines_this_turn = 4;
        g.finish_lock();
        assert_eq!(g.best_a, 13_200);
        assert_eq!(g.best_b, 2_000);
        g.game_type_b = true;
        g.setup();
        g.score = 3_000;
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!(g.best_b, 3_040);
        assert_eq!(g.best_a, 13_200);
    }
    #[test]
    fn select_specific_game_type_without_oscillation() {
        let mut g = Game::with_seed(12);
        g.scene = Scene::GameType;
        g.tick(RIGHT);
        assert!(g.game_type_b);
        g.tick(0);
        g.tick(RIGHT);
        assert!(g.game_type_b);
        g.tick(0);
        g.tick(LEFT);
        assert!(!g.game_type_b);
    }
    #[test]
    fn hidden_level_selection_holding_a_plus_start() {
        let mut g = Game::with_seed(12);
        g.scene = Scene::LevelSelect;
        g.start_level = 9;
        g.tick(A | START);
        assert_eq!(g.start_level, 19);
        assert_eq!(g.level, 19);
        assert_eq!(g.scene, Scene::Playing);
    }
    #[test]
    fn pause_resumes_without_gravity() {
        let mut g = game();
        let y = g.y;
        g.tick(START);
        assert_eq!(g.scene, Scene::Paused);
        for _ in 0..60 {
            g.tick(0);
        }
        assert_eq!(g.y, y);
        g.tick(START);
        assert_eq!(g.scene, Scene::Playing);
    }
    #[test]
    fn rng_matches_nes_lfsr_vectors() {
        let mut g = Game::with_seed(0x8889);
        for expected in [
            0xc444, 0x6222, 0x3111, 0x9808, 0x4c04, 0x2602, 0x1301, 0x8980, 0x4440, 0x2220,
        ] {
            g.advance_rng();
            assert_eq!(g.rng, expected);
        }
    }
    #[test]
    fn piece_selection_matches_reference_vector() {
        let mut g = Game::with_seed(0x8889);
        let mut actual = Vec::new();
        for _ in 0..12 {
            g.advance_rng();
            actual.push(g.pick_piece());
        }
        assert_eq!(actual, [14, 11, 11, 2, 10, 18, 11, 2, 7, 18, 14, 11]);
    }
    #[test]
    fn fallback_uses_previous_orientation_not_index() {
        let mut g = Game::with_seed(1);
        g.previous_piece = 18;
        g.piece_counter = 0;
        g.rng = 6; // 6+1 = 7: reroll
        g.advance_rng();
        let expected = SPAWN[((g.rng as usize & 7) + 18) % 7] as usize;
        g.rng = 6;
        assert_eq!(g.pick_piece(), expected);
    }
    #[test]
    fn gravity_matches_every_ntsc_level_and_above_29() {
        for level in [0, 1, 8, 9, 10, 12, 13, 15, 16, 18, 19, 28, 29, 30, 99] {
            let mut g = game();
            g.level = level;
            g.gravity_timer = 0;
            let speed = GRAVITY[level.min(29) as usize];
            for _ in 1..speed {
                g.tick(0);
                assert_eq!(g.y, 0, "level {level}");
            }
            g.tick(0);
            assert_eq!(g.y, 1, "level {level}");
        }
    }
    #[test]
    fn collision_with_stack_prevents_move_and_rotation() {
        let mut g = game();
        g.kind = 17;
        g.x = 5;
        g.y = 5;
        g.field[5][3] = 1;
        assert!(!g.rotate(true));
        assert_eq!(g.kind, 17);
        g.field[6][6] = 1;
        assert!(!g.shift(1));
        assert_eq!(g.x, 5);
    }
    #[test]
    fn lock_copies_four_cells_and_delays_entry() {
        let mut g = game();
        g.kind = 10;
        g.x = 5;
        g.y = 18;
        assert!(!g.down_step());
        assert_eq!(g.field.iter().flatten().filter(|&&v| v != 0).count(), 4);
        assert_eq!(g.entry_delay, 18);
        for _ in 0..17 {
            g.tick(0);
            assert_eq!(g.entry_delay > 0, true);
        }
        g.tick(0);
        assert_eq!(g.entry_delay, 0);
        assert_eq!(g.y, 0);
    }
    #[test]
    fn das_initial_repeat_wall_charge_and_preservation() {
        let mut g = game();
        g.kind = 10;
        g.x = 5;
        g.y = 5;
        g.tick(LEFT);
        assert_eq!(g.x, 4);
        for _ in 0..15 {
            g.tick(LEFT);
        }
        assert_eq!(g.x, 4);
        g.tick(LEFT);
        assert_eq!(g.x, 3);
        for _ in 0..6 {
            g.tick(LEFT);
        }
        assert_eq!(g.x, 2);
        for _ in 0..12 {
            g.tick(LEFT);
        }
        assert_eq!(g.x, 1);
        assert_eq!(g.das_counter, 16);
        g.spawn();
        assert_eq!(g.das_counter, 16);
        let x = g.x;
        g.tick(LEFT);
        assert_eq!(g.x, x - 1);
    }
    #[test]
    fn soft_drop_requires_new_press_and_cancel_discards_points() {
        let mut g = game();
        g.kind = 10;
        g.tick(DOWN);
        g.tick(DOWN);
        assert_eq!(g.y, 0);
        g.tick(DOWN);
        assert_eq!(g.y, 1);
        assert_eq!(g.soft_score, 1);
        g.tick(0);
        assert_eq!(g.soft_score, 0);
        g.tick(DOWN);
        g.tick(DOWN);
        g.tick(DOWN);
        g.tick(DOWN);
        g.tick(DOWN);
        assert_eq!(g.soft_score, 2);
        g.finish_lock();
        assert_eq!(g.score, 1);
    }
    #[test]
    fn all_clear_sizes_use_original_points() {
        for (count, points) in [(1, 40), (2, 100), (3, 300), (4, 1200)] {
            let mut g = game();
            g.level = 3;
            for y in H - count..H {
                g.field[y] = [1; W];
            }
            g.lines_this_turn = count as u8;
            g.finish_lock();
            assert_eq!(g.score, points * 4);
            assert_eq!(g.lines, count as u32);
            assert_eq!(g.stats[count - 1], 1);
        }
    }
    #[test]
    fn level_transition_precedes_scoring_and_repeats_after_high_start() {
        let mut g = game();
        g.start_level = 15;
        g.level = 15;
        g.lines = 99;
        g.field[19] = [1; W];
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!((g.lines, g.level, g.score), (100, 16, 680));
        g.lines = 109;
        g.field[19] = [1; W];
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!((g.lines, g.level, g.score), (110, 17, 1400));
        let mut g = game();
        g.start_level = 19;
        g.level = 19;
        g.lines = 139;
        g.field[19] = [1; W];
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!((g.level, g.score), (20, 840));
    }
    #[test]
    fn type_b_hidden_level_bonus_uses_displayed_units_digit() {
        let mut g = game();
        g.game_type_b = true;
        g.start_level = 19;
        g.b_height = 5;
        g.setup();
        g.lines = 1;
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!(g.scene, Scene::Won);
        assert_eq!(g.score, 14_800);
    }
    #[test]
    fn victory_bonus_is_counted_over_time_before_exit() {
        let mut g = game();
        g.game_type_b = true;
        g.start_level = 1;
        g.b_height = 1;
        g.setup();
        g.lines = 1;
        g.lines_this_turn = 1;
        g.finish_lock();
        assert_eq!(g.win_display(), (1000, 1000, 80));
        g.tick(START);
        assert_eq!(g.scene, Scene::Won);
        for _ in 1..g.win_duration() {
            g.tick(0);
        }
        assert_eq!(g.win_display(), (0, 0, 2080));
        g.tick(START);
        assert_eq!(g.scene, Scene::NameEntry);
    }
    #[test]
    fn score_persistence_abi_keeps_mode_scores_separate() {
        let mut g = game();
        unsafe {
            let ptr = &mut g as *mut Game;
            tr_set_best(ptr, 54_321, 12_345);
            assert_eq!(tr_best(ptr, false), 54_321);
            assert_eq!(tr_best(ptr, true), 12_345);
            tr_set_best(ptr, u32::MAX, 12_345);
            assert_eq!(tr_best(ptr, false), 999_999);
        }
    }
    #[test]
    fn record_entry_breaks_ties_and_survives_abi_roundtrip() {
        let mut g = game();
        g.score = 7_500;
        g.level = 8;
        g.scene = Scene::GameOver;
        g.game_over_frame = 143;
        g.tick(START);
        assert_eq!(g.scene, Scene::NameEntry);
        assert_eq!(g.record_row, 1);
        assert_eq!(g.records[0][1].score, 7_500);
        assert_eq!(g.records[0][2].score, 7_500);
        g.records[0][1].name = [1, 2, 3, 27, 36, 43];
        let packed = unsafe { tr_record(&g, false, 1) };
        let mut loaded = Game::with_seed(1);
        unsafe {
            tr_set_record(&mut loaded, false, 1, packed);
        }
        assert_eq!(loaded.records[0][1], g.records[0][1]);
        assert_eq!(loaded.records[1][1].score, 1_000);
    }
    #[test]
    fn all_type_b_heights_keep_only_allowed_rows_and_have_a_hole() {
        let starts = [20, 17, 15, 12, 10, 8];
        for height in 0..=5 {
            let mut g = Game::with_seed(0x8889);
            g.game_type_b = true;
            g.b_height = height;
            g.setup();
            assert!(g.field[..starts[height as usize]]
                .iter()
                .flatten()
                .all(|&x| x == 0));
            assert!(g.field.iter().all(|row| row.contains(&0)));
            assert_eq!(g.lines, 25);
        }
    }
    #[test]
    fn android_focus_loss_pauses_and_requires_explicit_resume() {
        let mut g = game();
        let y = g.y;
        g.pause_for_focus_loss();
        assert_eq!(g.scene, Scene::Paused);
        for _ in 0..100 {
            g.tick(0);
        }
        assert_eq!(g.y, y);
        g.tick(START);
        assert_eq!(g.scene, Scene::Playing);
    }
    #[test]
    fn type_b_menu_selects_height_and_clamps_grid() {
        let mut g = Game::with_seed(5);
        g.scene = Scene::LevelSelect;
        g.game_type_b = true;
        g.tick(DOWN);
        assert_eq!(g.start_level, 5);
        g.tick(0);
        g.tick(A);
        assert!(g.selecting_height);
        g.tick(0);
        g.tick(DOWN);
        assert_eq!(g.b_height, 3);
        g.tick(0);
        g.tick(RIGHT);
        assert_eq!(g.b_height, 4);
        g.tick(0);
        g.tick(UP);
        assert_eq!(g.b_height, 1);
    }
}
