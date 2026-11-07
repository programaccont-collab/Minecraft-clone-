use macroquad::audio::{self, PlaySoundParams, Sound};
use macroquad::prelude::*;
use macroquad::rand;
use std::collections::HashMap;

const CS: i32 = 16; // chunk width/depth
const CH: i32 = 48; // chunk height
const BASE: i32 = 20;
const SEA: i32 = 19;
const TILES: f32 = 19.0;
const NAMES: [&str; 18] = [
    "grass", "dirt", "stone", "sand", "wood", "leaves", "planks", "brick", "water", "lava", "snow", "moss",
    "spawner", "stick", "sword", "pickaxe", "apple", "gel",
];
const TERRAIN: [&str; 3] = ["Flat", "Hills", "Mountains"];
const TREES: [&str; 3] = ["Off", "Few", "Many"];
const BIOME_NAMES: [&str; 5] = ["Plains", "Desert", "Snowy", "Forest", "Lush"];

type Cell = (i32, i32, i32);

// ---------------------------------------------------------------- settings
#[derive(Clone)]
struct Settings {
    seed: u32,
    terrain: usize,
    trees: usize,
    render: i32,
    fov: f32,
    shading: bool,
    show_fps: bool,
    day_night: bool,
    mobs: usize,
    volume: f32,
    sens: f32,
    survival: bool,
}

// ---------------------------------------------------------------- noise
fn hash2(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (z as u32).wrapping_mul(668265263)
        ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    hash2(x ^ y.wrapping_mul(7919), z.wrapping_add(y.wrapping_mul(104729)), seed)
}

fn vnoise(x: f32, z: f32, seed: u32) -> f32 {
    let (xf, zf) = (x.floor(), z.floor());
    let (fx, fz) = (x - xf, z - zf);
    let (sx, sz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let (xi, zi) = (xf as i32, zf as i32);
    let a = hash2(xi, zi, seed);
    let b = hash2(xi + 1, zi, seed);
    let c = hash2(xi, zi + 1, seed);
    let d = hash2(xi + 1, zi + 1, seed);
    let t = a + (b - a) * sx;
    let u = c + (d - c) * sx;
    t + (u - t) * sz
}

fn lerp1(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn vnoise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xf, yf, zf) = (x.floor(), y.floor(), z.floor());
    let (sx, sy, sz) = (smooth(x - xf), smooth(y - yf), smooth(z - zf));
    let (xi, yi, zi) = (xf as i32, yf as i32, zf as i32);
    let c = |dx: i32, dy: i32, dz: i32| hash3(xi + dx, yi + dy, zi + dz, seed);
    let x00 = lerp1(c(0, 0, 0), c(1, 0, 0), sx);
    let x10 = lerp1(c(0, 1, 0), c(1, 1, 0), sx);
    let x01 = lerp1(c(0, 0, 1), c(1, 0, 1), sx);
    let x11 = lerp1(c(0, 1, 1), c(1, 1, 1), sx);
    let y0 = lerp1(x00, x10, sy);
    let y1 = lerp1(x01, x11, sy);
    lerp1(y0, y1, sz)
}

fn height(x: i32, z: i32, s: &Settings) -> i32 {
    let (fx, fz) = (x as f32, z as f32);
    let n = vnoise(fx / 40.0, fz / 40.0, s.seed)
        + vnoise(fx / 18.0, fz / 18.0, s.seed ^ 1) * 0.5
        + vnoise(fx / 8.0, fz / 8.0, s.seed ^ 2) * 0.25;
    let n = n / 1.75;
    let amp = [1.5f32, 7.0, 15.0][s.terrain];
    (BASE as f32 + (n - 0.5) * 2.0 * amp).round() as i32
}

// 0 plains, 1 desert, 2 snowy, 3 forest, 4 lush
fn biome(x: i32, z: i32, s: &Settings) -> usize {
    let t = vnoise(x as f32 / 110.0, z as f32 / 110.0, s.seed ^ 11);
    let m = vnoise(x as f32 / 90.0, z as f32 / 90.0, s.seed ^ 12);
    if t < 0.38 {
        2
    } else if t > 0.62 && m < 0.5 {
        1
    } else if m > 0.64 {
        4
    } else if m > 0.55 {
        3
    } else {
        0
    }
}

// ---------------------------------------------------------------- blocks / meshing
fn idx(x: i32, y: i32, z: i32) -> usize {
    (x + z * CS + y * CS * CS) as usize
}

fn is_liquid(b: u8) -> bool {
    b == 9 || b == 10
}

fn tile(b: u8, face: usize) -> f32 {
    let t: i32 = match b {
        1 => [0, 2, 1][face],
        x if x >= 2 && x <= 18 => x as i32,
        _ => 0,
    };
    t as f32
}

struct Face {
    n: (i32, i32, i32),
    kind: usize, // 0 top, 1 bottom, 2 side
    shade: f32,
    v: [[f32; 3]; 4],
}

const FACES: [Face; 6] = [
    Face { n: (1, 0, 0), kind: 2, shade: 0.8, v: [[0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5]] },
    Face { n: (-1, 0, 0), kind: 2, shade: 0.8, v: [[-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5]] },
    Face { n: (0, 1, 0), kind: 0, shade: 1.0, v: [[-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5]] },
    Face { n: (0, -1, 0), kind: 1, shade: 0.55, v: [[-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5]] },
    Face { n: (0, 0, 1), kind: 2, shade: 0.7, v: [[0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, -0.5, 0.5]] },
    Face { n: (0, 0, -1), kind: 2, shade: 0.7, v: [[-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5]] },
];
const UVS: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

struct MeshBuf {
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    out: Vec<Mesh>,
}

impl MeshBuf {
    fn new() -> MeshBuf {
        MeshBuf { verts: Vec::new(), inds: Vec::new(), out: Vec::new() }
    }

    fn flush(&mut self, atlas: &Texture2D) {
        if !self.verts.is_empty() {
            self.out.push(Mesh {
                vertices: std::mem::take(&mut self.verts),
                indices: std::mem::take(&mut self.inds),
                texture: Some(atlas.clone()),
            });
        }
    }

    fn quad(&mut self, atlas: &Texture2D, wx: f32, wy: f32, wz: f32, f: &Face, tl: f32, col: Color) {
        if self.verts.len() + 4 > 65000 {
            self.flush(atlas);
        }
        let base = self.verts.len() as u16;
        for k in 0..4 {
            let p = f.v[k];
            let u = (tl + 0.002 + UVS[k][0] * 0.996) / TILES;
            let v = 0.002 + UVS[k][1] * 0.996;
            self.verts.push(Vertex::new(wx + p[0], wy + p[1], wz + p[2], u, v, col));
        }
        self.inds.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

struct Chunk {
    b: Vec<u8>,
    dirty: bool,
    meshes: Vec<Mesh>,
    wmeshes: Vec<Mesh>,
}

struct World {
    chunks: HashMap<(i32, i32), Chunk>,
    edits: HashMap<(i32, i32), HashMap<Cell, u8>>,
    spawners: Vec<Cell>,
}

impl World {
    fn new() -> World {
        World { chunks: HashMap::new(), edits: HashMap::new(), spawners: Vec::new() }
    }

    fn has(&self, cx: i32, cz: i32) -> bool {
        self.chunks.contains_key(&(cx, cz))
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if y < 0 || y >= CH {
            return 0;
        }
        match self.chunks.get(&(x.div_euclid(CS), z.div_euclid(CS))) {
            Some(c) => c.b[idx(x.rem_euclid(CS), y, z.rem_euclid(CS))],
            None => 0,
        }
    }

    // solid for collision: unloaded chunks and the floor count as solid; liquids are not solid
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        if y < 0 {
            return true;
        }
        if y >= CH {
            return false;
        }
        match self.chunks.get(&(x.div_euclid(CS), z.div_euclid(CS))) {
            Some(c) => {
                let t = c.b[idx(x.rem_euclid(CS), y, z.rem_euclid(CS))];
                t != 0 && !is_liquid(t)
            }
            None => true,
        }
    }

    // highest non-liquid block of a column (surface)
    fn top(&self, x: i32, z: i32) -> Option<i32> {
        let c = self.chunks.get(&(x.div_euclid(CS), z.div_euclid(CS)))?;
        let (lx, lz) = (x.rem_euclid(CS), z.rem_euclid(CS));
        for y in (0..CH).rev() {
            let t = c.b[idx(lx, y, lz)];
            if t != 0 && !is_liquid(t) {
                return Some(y);
            }
        }
        Some(0)
    }

    // first non-liquid block at or below y
    fn ground(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let c = self.chunks.get(&(x.div_euclid(CS), z.div_euclid(CS)))?;
        let (lx, lz) = (x.rem_euclid(CS), z.rem_euclid(CS));
        let mut yy = y.min(CH - 1);
        while yy >= 0 {
            let t = c.b[idx(lx, yy, lz)];
            if t != 0 && !is_liquid(t) {
                return Some(yy);
            }
            yy -= 1;
        }
        Some(0)
    }

    fn mark(&mut self, cx: i32, cz: i32) {
        if let Some(c) = self.chunks.get_mut(&(cx, cz)) {
            c.dirty = true;
        }
    }

    fn mark_all(&mut self) {
        for c in self.chunks.values_mut() {
            c.dirty = true;
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, t: u8) {
        if y < 0 || y >= CH {
            return;
        }
        let (cx, cz) = (x.div_euclid(CS), z.div_euclid(CS));
        let (lx, lz) = (x.rem_euclid(CS), z.rem_euclid(CS));
        if let Some(c) = self.chunks.get_mut(&(cx, cz)) {
            c.b[idx(lx, y, lz)] = t;
            c.dirty = true;
        }
        self.edits.entry((cx, cz)).or_default().insert((lx, y, lz), t);
        if t == 13 && !self.spawners.contains(&(x, y, z)) {
            self.spawners.push((x, y, z));
        }
        if lx == 0 {
            self.mark(cx - 1, cz);
        }
        if lx == CS - 1 {
            self.mark(cx + 1, cz);
        }
        if lz == 0 {
            self.mark(cx, cz - 1);
        }
        if lz == CS - 1 {
            self.mark(cx, cz + 1);
        }
    }

    fn generate(&mut self, cx: i32, cz: i32, s: &Settings) {
        let mut b = vec![0u8; (CS * CS * CH) as usize];
        let mut heights = [0i32; 256];
        let mut bios = [0usize; 256];
        let mut found: Vec<Cell> = Vec::new();
        for lz in 0..CS {
            for lx in 0..CS {
                let (wx, wz) = (cx * CS + lx, cz * CS + lz);
                let bio = biome(wx, wz, s);
                let h = height(wx, wz, s);
                heights[(lx + lz * CS) as usize] = h;
                bios[(lx + lz * CS) as usize] = bio;
                let wet = h <= SEA + 1;
                let (top_b, sub_b): (u8, u8) = if wet {
                    (4, 4)
                } else {
                    match bio {
                        1 => (4, 4),
                        2 => (11, 2),
                        4 => (if hash2(wx, wz, s.seed ^ 5) < 0.5 { 12 } else { 1 }, 2),
                        _ => (1, 2),
                    }
                };
                for y in 0..=h {
                    let t = if y == h {
                        top_b
                    } else if y >= h - 3 {
                        sub_b
                    } else {
                        3
                    };
                    b[idx(lx, y, lz)] = t;
                }
                if h < SEA {
                    for y in (h + 1)..=SEA {
                        b[idx(lx, y, lz)] = 9;
                    }
                }
                // caves, lava lakes, lush pools, dungeon spawners
                if h > SEA + 2 {
                    for y in 3..=h {
                        let thr = if y >= h - 3 { 0.78 } else { 0.72 };
                        if vnoise3(wx as f32 / 10.0, y as f32 / 7.0, wz as f32 / 10.0, s.seed ^ 21) > thr {
                            let lush = bio == 4;
                            b[idx(lx, y, lz)] = if y <= 5 {
                                10
                            } else if lush && y <= 11 {
                                9
                            } else {
                                0
                            };
                            if b[idx(lx, y, lz)] == 0 && b[idx(lx, y - 1, lz)] == 3 {
                                if hash3(wx, y, wz, s.seed ^ 41) < 0.004 {
                                    b[idx(lx, y, lz)] = 13;
                                    found.push((wx, y, wz));
                                } else if lush && hash3(wx, y, wz, s.seed ^ 31) < 0.6 {
                                    b[idx(lx, y - 1, lz)] = 12;
                                }
                            }
                        }
                    }
                }
            }
        }
        // trees
        let f = [0.0f32, 1.0, 2.5][s.trees];
        let per_biome = [0.008f32, 0.0, 0.008, 0.035, 0.03];
        for lz in 2..(CS - 2) {
            for lx in 2..(CS - 2) {
                let i = (lx + lz * CS) as usize;
                let h = heights[i];
                let dens = per_biome[bios[i]] * f;
                if dens <= 0.0 || h <= SEA + 1 {
                    continue;
                }
                let tb = b[idx(lx, h, lz)];
                if tb != 1 && tb != 12 && tb != 11 {
                    continue;
                }
                if hash2(cx * CS + lx, cz * CS + lz, s.seed ^ 77) < dens {
                    for dy in 3..=4 {
                        for dx in -1..=1 {
                            for dz in -1..=1 {
                                b[idx(lx + dx, h + dy, lz + dz)] = 6;
                            }
                        }
                    }
                    b[idx(lx, h + 5, lz)] = 6;
                    for dy in 1..=4 {
                        b[idx(lx, h + dy, lz)] = 5;
                    }
                }
            }
        }
        if let Some(e) = self.edits.get(&(cx, cz)) {
            for (&(lx, y, lz), &t) in e.iter() {
                b[idx(lx, y, lz)] = t;
            }
        }
        for c in found {
            if !self.spawners.contains(&c) {
                self.spawners.push(c);
            }
        }
        self.chunks.insert((cx, cz), Chunk { b, dirty: true, meshes: Vec::new(), wmeshes: Vec::new() });
    }

    fn build_mesh(&self, cx: i32, cz: i32, atlas: &Texture2D, shading: bool) -> (Vec<Mesh>, Vec<Mesh>) {
        let c = match self.chunks.get(&(cx, cz)) {
            Some(c) => c,
            None => return (Vec::new(), Vec::new()),
        };
        let mut solid_buf = MeshBuf::new();
        let mut water_buf = MeshBuf::new();
        for y in 0..CH {
            for lz in 0..CS {
                for lx in 0..CS {
                    let t = c.b[idx(lx, y, lz)];
                    if t == 0 {
                        continue;
                    }
                    for f in FACES.iter() {
                        let (nx, ny, nz) = (lx + f.n.0, y + f.n.1, lz + f.n.2);
                        let nb: u8 = if ny < 0 {
                            3
                        } else if ny >= CH {
                            0
                        } else if nx >= 0 && nx < CS && nz >= 0 && nz < CS {
                            c.b[idx(nx, ny, nz)]
                        } else {
                            self.get(cx * CS + nx, ny, cz * CS + nz)
                        };
                        let wx = (cx * CS + lx) as f32;
                        let wy = y as f32;
                        let wz = (cz * CS + lz) as f32;
                        if t == 9 {
                            if nb != 0 {
                                continue;
                            }
                            water_buf.quad(atlas, wx, wy, wz, f, tile(t, f.kind), Color::new(1.0, 1.0, 1.0, 0.62));
                        } else {
                            if nb != 0 && nb != 9 {
                                continue;
                            }
                            let sh = if shading && t != 10 { f.shade } else { 1.0 };
                            solid_buf.quad(atlas, wx, wy, wz, f, tile(t, f.kind), Color::new(sh, sh, sh, 1.0));
                        }
                    }
                }
            }
        }
        solid_buf.flush(atlas);
        water_buf.flush(atlas);
        (solid_buf.out, water_buf.out)
    }
}

// ---------------------------------------------------------------- helpers
fn cell(p: Vec3) -> Cell {
    ((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32, (p.z + 0.5).floor() as i32)
}

fn hits(w: &World, f: Vec3) -> bool {
    let e = 0.001;
    let x0 = (f.x - 0.3 + 0.5 + e).floor() as i32;
    let x1 = (f.x + 0.3 + 0.5 - e).floor() as i32;
    let y0 = (f.y + 0.5 + e).floor() as i32;
    let y1 = (f.y + 1.8 + 0.5 - e).floor() as i32;
    let z0 = (f.z - 0.3 + 0.5 + e).floor() as i32;
    let z1 = (f.z + 0.3 + 0.5 - e).floor() as i32;
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                if w.solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

fn raycast(w: &World, o: Vec3, d: Vec3, max: f32) -> Option<(Cell, Cell, f32)> {
    let mut prev = cell(o);
    let mut t = 0.0f32;
    while t < max {
        let c = cell(o + d * t);
        let b = w.get(c.0, c.1, c.2);
        if b != 0 && !is_liquid(b) {
            return Some((c, prev, t));
        }
        prev = c;
        t += 0.04;
    }
    None
}

struct Blob {
    pos: Vec3,
    base: f32,
    dir: Vec2,
    timer: f32,
    hop: f32,
    hp: i32,
    hurt: f32,
}

fn mob_hit(mobs: &[Blob], o: Vec3, d: Vec3, max: f32) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, m) in mobs.iter().enumerate() {
        let v = m.pos - o;
        let t = v.dot(d);
        if t < 0.0 || t > max {
            continue;
        }
        if (v - d * t).length() < 0.6 {
            let better = match best {
                Some((_, bt)) => t < bt,
                None => true,
            };
            if better {
                best = Some((i, t));
            }
        }
    }
    best
}

fn drop_of(b: u8, pick: bool) -> Option<u8> {
    match b {
        1 => Some(2),
        3 | 8 => {
            if pick {
                Some(b)
            } else {
                None
            }
        }
        6 => {
            if rand::gen_range(0.0f32, 1.0) < 0.2 {
                Some(17)
            } else {
                None
            }
        }
        9 | 10 | 13 => None,
        _ => Some(b),
    }
}

fn play_snd(s: &Sound, vol: f32) {
    audio::play_sound(s, PlaySoundParams { looped: false, volume: vol });
}

fn get_taps() -> Vec<Vec2> {
    let mut v: Vec<Vec2> = Vec::new();
    for t in touches() {
        if t.phase == TouchPhase::Started {
            v.push(t.position);
        }
    }
    if cfg!(not(target_os = "android")) && is_mouse_button_pressed(MouseButton::Left) {
        v.push(Vec2::from(mouse_position()));
    }
    v
}

fn button(r: Rect, label: &str, size: f32, taps: &[Vec2]) -> bool {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.15, 0.15, 0.22, 0.85));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, WHITE);
    let d = measure_text(label, None, size as u16, 1.0);
    draw_text(label, r.x + r.w / 2.0 - d.width / 2.0, r.y + r.h / 2.0 + d.height / 2.0, size, WHITE);
    taps.iter().any(|t| r.contains(*t))
}

fn centered(text: &str, y: f32, size: f32, color: Color) {
    let d = measure_text(text, None, size as u16, 1.0);
    draw_text(text, screen_width() / 2.0 - d.width / 2.0, y, size, color);
}

fn title(text: &str, y: f32, size: f32) {
    let d = measure_text(text, None, size as u16, 1.0);
    let x = screen_width() / 2.0 - d.width / 2.0;
    draw_text(text, x + 4.0, y + 4.0, size, BLACK);
    draw_text(text, x, y, size, WHITE);
}

// one settings row: returns -1 / 0 / +1
fn row(x: f32, y: f32, cw: f32, rh: f32, label: &str, value: &str, lab: (&str, &str), on: bool, taps: &[Vec2]) -> i32 {
    let bg = if on { Color::new(0.2, 0.2, 0.28, 1.0) } else { Color::new(0.14, 0.14, 0.16, 1.0) };
    let tc = if on { WHITE } else { GRAY };
    draw_rectangle(x, y + 4.0, cw, rh - 8.0, bg);
    draw_text(label, x + 14.0, y + rh / 2.0 + 8.0, 30.0, tc);
    let bw = rh * 1.15;
    let bh = rh - 16.0;
    let minus = Rect::new(x + cw - bw * 2.0 - 24.0, y + 8.0, bw, bh);
    let plus = Rect::new(x + cw - bw - 10.0, y + 8.0, bw, bh);
    let vd = measure_text(value, None, 30, 1.0);
    draw_text(value, minus.x - 10.0 - vd.width, y + rh / 2.0 + 8.0, 30.0, tc);
    let empty: &[Vec2] = &[];
    let t = if on { taps } else { empty };
    let m = button(minus, lab.0, 26.0, t);
    let p = button(