//! Native (not 6502-emulated) NES Tetris logic + pixel renderer.
//! Tables were manually translated from the user-supplied TetrisNESDisasm archive.
//! 60 Hz tick; game graphics are original 8×8 assets extracted from the archive.
//! Audio is intentionally omitted.

use std::{slice, time::{SystemTime, UNIX_EPOCH}};

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
    [(0,-1),(0,0),(0,1),(-1,0)],          // 0 T up
    [(-1,0),(0,0),(0,1),(1,0)],          // 1 T right
    [(0,-1),(0,0),(0,1),(1,0)],           // 2 T down (spawn)
    [(-1,0),(0,-1),(0,0),(1,0)],         // 3 T left
    [(-1,0),(0,0),(1,-1),(1,0)],         // 4 J left
    [(-1,-1),(0,-1),(0,0),(0,1)],        // 5 J up
    [(-1,0),(-1,1),(0,0),(1,0)],         // 6 J right
    [(0,-1),(0,0),(0,1),(1,1)],          // 7 J down (spawn)
    [(0,-1),(0,0),(1,0),(1,1)],          // 8 Z horiz (spawn)
    [(-1,1),(0,0),(0,1),(1,0)],          // 9 Z vert
    [(0,-1),(0,0),(1,-1),(1,0)],         // 10 O
    [(0,0),(0,1),(1,-1),(1,0)],          // 11 S horiz (spawn)
    [(-1,0),(0,0),(0,1),(1,1)],          // 12 S vert
    [(-1,0),(0,0),(1,0),(1,1)],          // 13 L right
    [(0,-1),(0,0),(0,1),(1,-1)],         // 14 L down (spawn)
    [(-1,-1),(-1,0),(0,0),(1,0)],        // 15 L left
    [(-1,1),(0,-1),(0,0),(0,1)],         // 16 L up
    [(-2,0),(-1,0),(0,0),(1,0)],         // 17 I vert
    [(0,-2),(0,-1),(0,0),(0,1)],         // 18 I horiz (spawn)
];

// Rotation table: [B = counterclockwise, A = clockwise].
const ROTATE: [[u8;2];19] = [
    [3,1],[0,2],[1,3],[2,0],
    [7,5],[4,6],[5,7],[6,4],
    [9,9],[8,8],[10,10],[12,12],[11,11],
    [16,14],[13,15],[14,16],[15,13],
    [18,18],[17,17],
];
const SPAWN: [u8;7] = [2, 7, 8, 10, 11, 14, 18]; // T J Z O S L I
const KIND: [u8;19] = [1,1,1,1, 2,2,2,2, 3,3, 4, 5,5, 6,6,6,6, 7,7];
const GRAVITY: [u8;30] = [
    48,43,38,33,28,23,18,13,8,6,
    5,5,5,4,4,4,3,3,3,2,
    2,2,2,2,2,2,2,2,2,1,
];
const POINTS: [u32;5] = [0,40,100,300,1200];

const GAME_GFX: &[u8;16384] = include_bytes!("../assets/game.raw");
const MENU_GFX: &[u8;16384] = include_bytes!("../assets/title_menu.raw");
const GAME_MAP: &[u8;1024] = include_bytes!("../assets/game.map");
const TITLE_MAP: &[u8;1024] = include_bytes!("../assets/title_screen.map");
const TYPE_MAP: &[u8;1024] = include_bytes!("../assets/game_type_menu.map");
const LEVEL_MAP: &[u8;1024] = include_bytes!("../assets/level_menu.map");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene { Title, GameType, LevelSelect, Playing, Paused, GameOver, Won }

pub struct Game {
    pub scene: Scene,
    pub field: [[u8; W];H],
    pub kind: usize,             // current orientation (0..19)
    pub next: usize,             // next spawn orientation
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
    frame: u64,
    last_input: u32,
    das_dir: i8,
    das_counter: u8,
    gravity_timer: u8,
    soft_timer: u8,
    soft_score: u32,
    line_animation: u8,
    lines_this_turn: u8,
    previous_piece: usize,
    piece_counter: u8,
    rng: u32,
}

impl Game {
    pub fn new() -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|t| t.as_nanos() as u32).unwrap_or(0x1234_5678);
        Self::with_seed(seed)
    }

    pub fn with_seed(seed: u32) -> Self {
        Self {
            scene: Scene::Title,
            field: [[0; W]; H], kind: 2, next: 2, x: 5, y: 0,
            lines: 0, score: 0, level: 0, start_level: 0,
            game_type_b: false, b_height: 0, show_next: true,
            best_a: 0, best_b: 0, stats: [0;4],
            frame: 0, last_input: 0, das_dir: 0, das_counter: 0,
            gravity_timer: 0, soft_timer: 0, soft_score: 0,
            line_animation: 0, lines_this_turn: 0,
            previous_piece: 0, piece_counter: 0,
            rng: if seed==0 {0x104c_73db} else {seed},
        }
    }

    fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn pick_piece(&mut self) -> usize {
        // Original pick algorithm: use (rng + spawnCount) & 7; if 7 or same
        // as previous use fallback (rng & 7 + previous) % 7. RNG implementation
        // itself is a native pseudorandom generator, not cycle-accurate NES RNG.
        self.piece_counter = self.piece_counter.wrapping_add(1);
        let mut index = ((self.random() as u8).wrapping_add(self.piece_counter) & 7) as usize;
        if index == 7 || index == self.previous_piece {
            index = (((self.random() & 7) as usize) + self.previous_piece) % 7;
        }
        self.previous_piece = index;
        SPAWN[index] as usize
    }

    fn setup(&mut self) {
        self.field = [[0;W];H];
        self.stats = [0;4];
        self.level = self.start_level;
        self.score = 0;
        self.lines = if self.game_type_b {25} else {0};
        self.line_animation=0;
        self.soft_score=0;
        self.gravity_timer=0;
        self.soft_timer=0;
        self.previous_piece=0;
        self.piece_counter=0;
        self.next=self.pick_piece();
        if self.game_type_b {self.generate_garbage();}
        self.scene=Scene::Playing;
        self.spawn();
    }

    fn generate_garbage(&mut self) {
        // Original Type-B layout: create 12 pseudo-random rows and then blank
        // the top according to height selection. The NES rngTable has 3/8
        // occupied cells using [blank,1,blank,2,3,3,blank,blank].
        let first_row = match self.b_height { 0=>20, 1=>17, 2=>15, 3=>12, 4=>10, _=>8 };
        for y in first_row..H {
            for x in 0..W {
                const RNG_TABLE:[u8;8]=[0,1,0,2,3,3,0,0];
                self.field[y][x]=RNG_TABLE[(self.random()&7) as usize];
            }
            // At least one blank in each row, matching typeBGuaranteeBlank.
            self.field[y][(self.random()%10) as usize]=0;
        }
    }

    fn spawn(&mut self) {
        self.kind = self.next;
        self.next = self.pick_piece();
        self.x=5;
        self.y=0;
        self.gravity_timer=0;
        self.soft_timer=0;
        self.soft_score=0;
        self.das_counter=0;
        self.das_dir=0;
        if !self.valid(self.x, self.y, self.kind) {self.scene=Scene::GameOver;}
    }

    fn valid(&self, x: i8,y: i8,kind: usize) -> bool {
        SHAPES[kind].iter().all(|&(dy,dx)| {
            let row=y as i16+dy as i16;
            let col=x as i16+dx as i16;
            col>=0 && col<W as i16 && row>=-2 && row<H as i16 &&
            (row<0 || self.field[row as usize][col as usize]==0)
        })
    }

    fn shift(&mut self, dx:i8) -> bool {
        let x = self.x+dx;
        if self.valid(x,self.y,self.kind) { self.x=x; true } else { false }
    }

    fn rotate(&mut self, clockwise:bool) -> bool {
        let new_kind=ROTATE[self.kind][usize::from(clockwise)] as usize;
        // No wall kicks, as in the original NES game.
        if self.valid(self.x,self.y,new_kind) {self.kind=new_kind; true} else {false}
    }

    fn down_step(&mut self) -> bool {
        if self.valid(self.x,self.y+1,self.kind) {self.y+=1;true}
        else {self.lock();false}
    }

    fn lock(&mut self) {
        let mut overflow=false;
        for &(dy,dx) in SHAPES[self.kind].iter() {
            let row=self.y+dy;
            let col=self.x+dx;
            if row<0 { overflow=true; }
            else if row<H as i8 && col>=0 && col<W as i8 {
                self.field[row as usize][col as usize]=KIND[self.kind];
            }
        }
        if overflow {self.scene=Scene::GameOver;return;}
        self.lines_this_turn=self.field.iter().filter(|row|row.iter().all(|&v|v!=0)).count() as u8;
        if self.lines_this_turn>0 {
            // Flash completed rows before collapsing them, not after.
            self.line_animation=20;
        } else {
            self.finish_lock();
        }
    }

    fn clear_lines(&mut self)->u8 {
        let mut write=H as isize-1;
        let mut removed=0;
        for read in (0..H).rev() {
            if self.field[read].iter().all(|&b|b!=0) { removed+=1; }
            else {
                self.field[write as usize] = self.field[read];
                write-=1;
            }
        }
        while write>=0 {self.field[write as usize]=[0;W];write-=1;}
        removed
    }

    fn finish_lock(&mut self) {
        let cleared=self.lines_this_turn as usize;
        if cleared>0 {
            self.clear_lines();
            self.stats[cleared.saturating_sub(1).min(3)] += 1;
        }
        // NES awards points using the level at the time of the clear.
        let gain=POINTS[cleared.min(4)]*(self.level+1);
        self.score=self.score.saturating_add(gain+self.soft_score).min(999_999);
        if self.game_type_b { self.best_b=self.best_b.max(self.score); }
        else { self.best_a=self.best_a.max(self.score); }
        if self.game_type_b {
            self.lines=self.lines.saturating_sub(self.lines_this_turn as u32);
            if self.lines==0 {
                // NES Type-B end-of-game level and height bonuses, without
                // the slow audio-timed counting animation.
                let bonus=(self.start_level + u32::from(self.b_height))*1_000;
                self.score=self.score.saturating_add(bonus).min(999_999);
                self.best_b=self.best_b.max(self.score);
                self.scene=Scene::Won;
                return;
            }
        } else {
            self.lines += self.lines_this_turn as u32;
            // NES NTSC threshold for starting level n: min(10(n+1), max(100,10n-50)).
            let initial_threshold = (self.start_level+1)*10;
            let threshold = initial_threshold.min((self.start_level*10).saturating_sub(50).max(100));
            if self.level==self.start_level {
                if self.lines>=threshold {self.level+=1;}
            } else if self.lines >= (self.level+1)*10 {
                self.level+=1;
            }
        }
        self.lines_this_turn=0;
        self.spawn();
    }

    /// Advance by one 60 Hz frame. Buttons are a bitmask and may be held across frames.
    pub fn tick(&mut self, input:u32) {
        self.frame=self.frame.wrapping_add(1);
        let pressed=input & !self.last_input;
        self.last_input=input;
        match self.scene {
            Scene::Title => {
                if pressed&(START|A)!=0 {self.scene=Scene::GameType;}
            }
            Scene::GameType => {
                if pressed&LEFT!=0 {self.game_type_b=false;}
                if pressed&RIGHT!=0 {self.game_type_b=true;}
                if pressed&(SELECT|A|B)!=0 {self.game_type_b=!self.game_type_b;}
                if pressed&START!=0 {self.scene=Scene::LevelSelect;}
            }
            Scene::LevelSelect => {
                if pressed&LEFT!=0 {self.start_level=(self.start_level+9)%10;}
                if pressed&RIGHT!=0 {self.start_level=(self.start_level+1)%10;}
                if self.game_type_b {
                    if pressed&UP!=0 {self.b_height=(self.b_height+1)%6;}
                    if pressed&DOWN!=0 {self.b_height=(self.b_height+5)%6;}
                }
                if pressed&START!=0 {
                    // NES hidden levels 10-19: hold A while pressing START.
                    if input&A!=0 {self.start_level+=10;}
                    self.setup();
                }
                if pressed&B!=0 {self.scene=Scene::GameType;}
            }
            Scene::Paused => {
                if pressed&START!=0 {self.scene=Scene::Playing;}
                if pressed&SELECT!=0 {self.scene=Scene::Title;}
            }
            Scene::GameOver|Scene::Won => {
                if pressed&START!=0 {self.scene=Scene::Title;}
            }
            Scene::Playing=>{
                if pressed&START!=0 {self.scene=Scene::Paused;return;}
                if pressed&SELECT!=0 {self.show_next=!self.show_next;}
                if self.line_animation>0 {
                    self.line_animation-=1;
                    if self.line_animation==0 {self.finish_lock();}
                    return;
                }
                if pressed&A!=0 {self.rotate(true);} else if pressed&B!=0 {self.rotate(false);}
                // NES NTSC DAS: immediate step, charge 16 frames, auto-repeat
                // approximately every 6 frames (counter resets to 10).
                let dir=if input&RIGHT!=0 && input&LEFT==0 {1}
                    else if input&LEFT!=0 && input&RIGHT==0 {-1}
                    else {0};
                if dir==0 || input&DOWN!=0 {
                    self.das_dir=0;self.das_counter=0;
                } else if dir!=self.das_dir {
                    self.das_dir=dir;self.das_counter=0;self.shift(dir);
                } else {
                    self.das_counter=self.das_counter.saturating_add(1);
                    if self.das_counter>=16 {
                        self.shift(dir);
                        self.das_counter=10;
                    }
                }
                self.gravity_timer=self.gravity_timer.saturating_add(1);
                let every=GRAVITY[self.level.min(29) as usize];
                if input&DOWN!=0 && input&(LEFT|RIGHT)==0 {
                    self.soft_timer=self.soft_timer.saturating_add(1);
                    if self.soft_timer>=2 {
                        self.soft_timer=0;
                        self.gravity_timer=0;
                        if self.down_step() {self.soft_score=self.soft_score.saturating_add(1);}
                        return;
                    }
                } else {self.soft_timer=0;}
                if self.gravity_timer>=every {
                    self.gravity_timer=0;
                    self.down_step();
                }
            }
        }
    }

    /// 256x240 ARGB pixel framebuffer, scaled externally by Android.
    pub fn render(&self, pixels:&mut [u32]) {
        if pixels.len() < NES_W*NES_H {return;}
        pixels[..NES_W*NES_H].fill(0xFF080817);
        let (gfx,map,pal):(&[u8;16384],&[u8;1024],&[[u32;4];4])= match self.scene {
            Scene::Title => (MENU_GFX,TITLE_MAP,&MENU_PALETTE),
            Scene::GameType => (MENU_GFX,TYPE_MAP,&MENU_PALETTE),
            Scene::LevelSelect => (MENU_GFX,LEVEL_MAP,&MENU_PALETTE),
            _ => (GAME_GFX,GAME_MAP,&GAME_PALETTE),
        };
        for row in 0..30 {
            for col in 0..32 {
                let tile=map[row*32+col];
                let attr=map[960+(row/4)*8+(col/4)];
                let shift=((row%4)/2*4+(col%4)/2*2) as u32;
                let pal_id=(attr>>shift)&3;
                draw_tile(pixels,(col*8) as i32,(row*8) as i32,gfx,tile,pal[pal_id as usize]);
            }
        }
        match self.scene {
            Scene::Title=>{},
            Scene::GameType=>{
                // Menu selection box, on top of original nametable.
                let x=if self.game_type_b {150} else {56};
                outline(pixels,x,56,80,24,0xFFFFC94D);
            }
            Scene::LevelSelect=>{
                let col=(self.start_level%5) as i32;
                let row=(self.start_level/5) as i32;
                outline(pixels,51+col*16,82+row*16,17,18,0xFFFFC94D);
                if self.game_type_b {
                    outline(pixels,155+(self.b_height%3) as i32*16,
                        82+(self.b_height/3) as i32*16,17,18,0xFF7AE5FF);
                } else {
                    shade_rect(pixels,101,22,58,15,0xFF090914);
                    draw_text(pixels,104,24,"A-TYPE",0xFFFFFFFF);
                }
            }
            _=>{
                // Aligned to the NES PPU screen layout (game field at x=96, y=48).
                for row in 0..H {
                    for col in 0..W {
                        let id=self.field[row][col];
                        if id!=0 {
                            let full=self.field[row].iter().all(|&v|v!=0);
                            if !(self.line_animation>0 && full && (self.line_animation/4)%2==0) {
                                draw_mino(pixels,(96+col*8) as i32,(48+row*8) as i32,id);
                            }
                        }
                    }
                }
                if matches!(self.scene,Scene::Playing|Scene::Paused) {
                    if self.line_animation==0 {
                        for &(dy,dx) in &SHAPES[self.kind] {
                            let row=self.y+dy;let col=self.x+dx;
                            if row>=0 && row<H as i8 && col>=0 && col<W as i8 {
                                draw_mino(pixels,96+col as i32*8,48+row as i32*8,KIND[self.kind]);
                            }
                        }
                    }
                }
                // Dynamic status at the original nametable text positions.
                draw_text(pixels,152,24,&format!("{:03}", self.lines.min(999)),0xFFFFC94D);
                draw_text(pixels,192,64,&format!("{:06}",self.score),0xFFFFC94D);
                draw_text(pixels,208,168,&format!("{:02}",self.level.min(99)),0xFFFFC94D);
                draw_text(pixels,16,96,&format!("{:06}",if self.game_type_b {self.best_b} else {self.best_a}),0xFFFFC94D);
                draw_text(pixels,8,208,&format!("1:{:03}",self.stats[0].min(999)),0xFFFFFFFF);
                draw_text(pixels,8,216,&format!("2:{:03}",self.stats[1].min(999)),0xFFFFFFFF);
                draw_text(pixels,8,224,&format!("3:{:03}",self.stats[2].min(999)),0xFFFFFFFF);
                draw_text(pixels,8,232,&format!("4:{:03}",self.stats[3].min(999)),0xFFFFFFFF);
                if self.show_next && matches!(self.scene,Scene::Playing|Scene::Paused) {
                    for &(dy,dx) in &SHAPES[self.next] {
                        draw_mino(pixels,204+dx as i32*8,122+dy as i32*8,KIND[self.next]);
                    }
                }
                match self.scene {
                    Scene::Paused => {
                        shade_rect(pixels,98,107,78,23,0xFF090B18);
                        draw_text(pixels,116,114,"PAUSE",0xFFFFFFFF);
                    }
                    Scene::GameOver => {
                        shade_rect(pixels,90,104,95,26,0xFF090B18);
                        draw_text(pixels,100,113,"GAME OVER",0xFFFFFFFF);
                    }
                    Scene::Won => {
                        shade_rect(pixels,90,104,95,26,0xFF090B18);
                        draw_text(pixels,112,113,"CLEAR!",0xFFFFFFFF);
                    }
                    _ => {}
                }
            }
        }
    }
}

// NES Tetris palette colors approximated for modern RGB display.
const GAME_PALETTE:[[u32;4];4]=[
    [0xFF090914,0xFFE7E7EB,0xFF2378D0,0xFFE84C50],
    [0xFF090914,0xFFCFCFCF,0xFF3B72D1,0xFF6A3BD0],
    [0xFF090914,0xFF66DEDF,0xFFE84C50,0xFF3FD35D],
    [0xFF090914,0xFFFFFFFF,0xFF6870A9,0xFFFFC444],
];
const MENU_PALETTE:[[u32;4];4]=[
    [0xFF090914,0xFFECECEC,0xFFADADAD,0xFF707070],
    [0xFF090914,0xFFEEEEEE,0xFFF1B4BC,0xFF6F84D7],
    [0xFF090914,0xFFECECEC,0xFF81BFD8,0xFFC65BA0],
    [0xFF090914,0xFFECECEC,0xFFD8B45D,0xFF566AA5],
];

fn draw_tile(pixels:&mut [u32],left:i32,top:i32,tiles:&[u8;16384],id:u8,palette:[u32;4]) {
    let offset=id as usize*64;
    for y in 0..8i32 {
        let dy=top+y;
        if !(0..NES_H as i32).contains(&dy) {continue;}
        for x in 0..8i32 {
            let dx=left+x;
            if !(0..NES_W as i32).contains(&dx) {continue;}
            let shade=tiles[offset+(y*8+x) as usize] as usize;
            pixels[dy as usize*NES_W+dx as usize]=palette[shade];
        }
    }
}
fn draw_mino(pixels:&mut [u32], x:i32,y:i32, kind:u8) {
    let (tile,pal) = match kind {
        1|4|7 => (0x7b, [0xFF090914,0xFFFFFFFF,0xFF6DAEF4,0xFF3476D6]),
        2|5 => (0x7d, [0xFF090914,0xFFFFFFFF,0xFF90EA9F,0xFF368A51]),
        _ => (0x7c, [0xFF090914,0xFFFFFFFF,0xFFF4AF73,0xFFCC555D]),
    };
    draw_tile(pixels,x,y,GAME_GFX,tile,pal);
}
fn draw_text(pixels:&mut [u32],x:i32,y:i32,text:&str,color:u32) {
    // 0..9 are the numbers in original game_tileset; letters A..Z at 0x0a..0x23.
    for (n,ch) in text.chars().enumerate() {
        let tile=match ch {
            '0'..='9' => ch as u8 - b'0',
            'A'..='Z' => 0x0a+ch as u8-b'A',
            '-' => 0x2c,
            '!' => 0x24,
            _ => 0xef,
        };
        draw_tile(pixels,x+n as i32*8,y,GAME_GFX,tile,
                  [0xFF090914,color,color,color]);
    }
}
fn outline(pixels:&mut [u32],x:i32,y:i32,w:i32,h:i32,color:u32) {
    for dy in 0..h {for dx in 0..w {
        if dy==0 || dy==h-1 || dx==0 || dx==w-1 {
            put(pixels,x+dx,y+dy,color);
        }
    }}
}
fn shade_rect(pixels:&mut [u32],x:i32,y:i32,w:i32,h:i32,color:u32) {
    for dy in 0..h {for dx in 0..w {put(pixels,x+dx,y+dy,color);}}
}
fn put(pixels:&mut [u32],x:i32,y:i32,color:u32) {
    if x>=0 && x<NES_W as i32 && y>=0 && y<NES_H as i32 {
        pixels[y as usize*NES_W+x as usize]=color;
    }
}

// Native C ABI - the only dependency of the NDK C++ JNI shim.
#[no_mangle]
pub extern "C" fn tr_create() -> *mut Game {Box::into_raw(Box::new(Game::new()))}

/// # Safety
/// `handle` must be a valid pointer previously returned by `tr_create`, and
/// must be released exactly once.
#[no_mangle]
pub unsafe extern "C" fn tr_destroy(handle:*mut Game) {
    if !handle.is_null() {drop(Box::from_raw(handle));}
}

/// # Safety
/// `handle` must point to a live `Game`.
#[no_mangle]
pub unsafe extern "C" fn tr_tick(handle:*mut Game,buttons:u32) {
    if let Some(game)=handle.as_mut(){game.tick(buttons);}
}

/// # Safety
/// `handle` must point to a live `Game`, and `out` must point to at least
/// 256×240 valid u32 pixels (ARGB_8888); `pixels_count` is checked.
#[no_mangle]
pub unsafe extern "C" fn tr_render(handle:*mut Game,out:*mut u32,pixels_count:usize) {
    if out.is_null() || pixels_count<NES_W*NES_H {return;}
    if let Some(game)=handle.as_ref() {
        game.render(slice::from_raw_parts_mut(out,NES_W*NES_H));
    }
}

/// Current score for persistence on the Android side.
#[no_mangle]
pub unsafe extern "C" fn tr_score(handle:*const Game)->u32 {
    handle.as_ref().map_or(0, |g|g.score)
}

/// Scene number: title=0, gameType=1, levelSelect=2, playing=3,
/// paused=4, gameOver=5, won=6.
#[no_mangle]
pub unsafe extern "C" fn tr_scene(handle:*const Game)->u32 {
    handle.as_ref().map_or(0, |g|match g.scene {
        Scene::Title=>0, Scene::GameType=>1, Scene::LevelSelect=>2,
        Scene::Playing=>3, Scene::Paused=>4, Scene::GameOver=>5, Scene::Won=>6,
    })
}
#[no_mangle]
pub unsafe extern "C" fn tr_game_type_b(handle:*const Game)->bool {
    handle.as_ref().is_some_and(|g|g.game_type_b)
}
#[no_mangle]
pub unsafe extern "C" fn tr_best(handle:*const Game, b:bool)->u32 {
    handle.as_ref().map_or(0, |g|if b {g.best_b} else {g.best_a})
}
#[no_mangle]
pub unsafe extern "C" fn tr_set_best(handle:*mut Game,a:u32,b:u32) {
    if let Some(g)=handle.as_mut() {g.best_a=a.min(999_999);g.best_b=b.min(999_999);}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game()->Game {let mut g=Game::with_seed(42);g.setup();g}
    #[test] fn all_piece_shapes_contain_four_unique_squares() {
        for s in SHAPES { for i in 0..4 {for j in i+1..4 {assert_ne!(s[i],s[j]);}} }
    }
    #[test] fn original_tables_are_intact() {
        assert_eq!(SPAWN,[2,7,8,10,11,14,18]);
        assert_eq!(GRAVITY[0],48);assert_eq!(GRAVITY[9],6);assert_eq!(GRAVITY[29],1);
        assert_eq!(ROTATE[2],[1,3]);
    }
    #[test] fn rotation_never_kicks_outside_board() {
        let mut g=game();g.kind=17;g.x=1;g.y=5;
        assert!(!g.rotate(false));assert_eq!(g.kind,17);
    }
    #[test] fn clears_multiple_rows_and_shifts() {
        let mut g=game();g.field[19]=[1;10];g.field[18]=[2;10];
        g.field[17][5]=7;
        assert_eq!(g.clear_lines(),2);
        assert_eq!(g.field[19][5],7);assert_eq!(g.field[18],[0;10]);
    }
    #[test] fn nes_scoring_and_level_transition() {
        let mut g=game();g.lines_this_turn=4;g.finish_lock();
        assert_eq!(g.score,1200);assert_eq!(g.lines,4);
        g.lines=8;g.lines_this_turn=2;g.finish_lock();
        assert_eq!(g.level,1);
    }
    #[test] fn game_over_when_occupied_spawn() {
        let mut g=game();g.field[0]=[1;10];g.field[1]=[1;10];g.spawn();
        assert_eq!(g.scene,Scene::GameOver);
    }
    #[test] fn renderer_writes_frame() {
        let g=game();let mut pixels=vec![0;256*240];g.render(&mut pixels);
        assert!(pixels.iter().any(|&x|x!=0xFF080817));
    }
    #[test] fn correct_type_b_height_garbage() {
        let mut g=Game::with_seed(123);
        g.game_type_b=true;
        g.b_height=0;
        g.setup();
        assert!(g.field.iter().all(|row|row.iter().all(|&x|x==0)));
        g.b_height=5;
        g.setup();
        assert!(g.field[0..8].iter().all(|row|row.iter().all(|&x|x==0)));
        assert!(g.field[8..20].iter().any(|row|row.iter().any(|&x|x!=0)));
        assert!(g.field.iter().all(|row|row.iter().any(|&x|x==0)));
    }
    #[test] fn finished_rows_flash_before_collapse() {
        let mut g=game();
        g.field[19]=[1;10];
        g.lines_this_turn=1;
        g.line_animation=1;
        g.tick(0);
        assert_eq!(g.field[19],[0;10]);
        assert_eq!(g.stats[0],1);
    }
    #[test] fn type_b_awards_level_height_bonus_on_goal() {
        let mut g=game();
        g.game_type_b=true;
        g.start_level=9;
        g.b_height=5;
        g.setup();
        g.lines=1;
        g.lines_this_turn=1;
        g.finish_lock();
        assert_eq!(g.scene,Scene::Won);
        assert_eq!(g.score,14_400);
    }
    #[test] fn best_scores_independent_of_game_modes() {
        let mut g=game();
        g.lines_this_turn=4;g.finish_lock();
        assert_eq!(g.best_a,1200); assert_eq!(g.best_b,0);
        g.game_type_b=true;g.setup();g.lines_this_turn=1;g.finish_lock();
        assert_eq!(g.best_b,40); assert_eq!(g.best_a,1200);
    }
    #[test] fn select_specific_game_type_without_oscillation() {
        let mut g=Game::with_seed(12);
        g.scene=Scene::GameType;
        g.tick(RIGHT);
        assert!(g.game_type_b);
        g.tick(0);
        g.tick(RIGHT);
        assert!(g.game_type_b);
        g.tick(0);
        g.tick(LEFT);
        assert!(!g.game_type_b);
    }
    #[test] fn hidden_level_selection_holding_a_plus_start() {
        let mut g=Game::with_seed(12);
        g.scene=Scene::LevelSelect;
        g.start_level=9;
        g.tick(A|START);
        assert_eq!(g.start_level,19);
        assert_eq!(g.level,19);
        assert_eq!(g.scene,Scene::Playing);
    }
    #[test] fn pause_resumes_without_gravity() {
        let mut g=game();let y=g.y;g.tick(START);assert_eq!(g.scene,Scene::Paused);
        for _ in 0..60 {g.tick(0);}assert_eq!(g.y,y);
        g.tick(START);assert_eq!(g.scene,Scene::Playing);
    }
}
