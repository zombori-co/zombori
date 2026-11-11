//! The celebration head as a small 3D model, rendered on the CPU straight into the pixel-art
//! screen. The shape is a signed distance field (each part is a simple solid: ellipsoids for the
//! skull, jaw and nose, tapered capsules for the hair spikes, discs and rings for the glasses), and
//! each screen pixel marches a ray into it ("sphere tracing"). Lighting is quantized into short
//! color ramps so it stays pixel art. No hardware involved, so this runs (and is tested) anywhere.
//!
//! Units are roughly virtual-screen pixels. Head space: x to the viewer's right, y up, z toward the
//! viewer; the origin is the middle of the skull.

use theme::{Rgb, VIRTUAL_H, VIRTUAL_W};

#[derive(Clone, Copy, Debug, PartialEq)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

const fn v(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    fn add(self, o: V3) -> V3 {
        v(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    fn sub(self, o: V3) -> V3 {
        v(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    fn mul(self, k: f32) -> V3 {
        v(self.x * k, self.y * k, self.z * k)
    }
    fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn norm(self) -> V3 {
        self.mul(1.0 / self.len().max(1e-6))
    }
    /// Rotate about the x axis: positive tips the top of the head toward the viewer (a nod).
    fn pitch(self, angle: f32) -> V3 {
        let (s, c) = angle.sin_cos();
        v(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    /// Rotate about the y axis: positive turns the face toward the viewer's right.
    fn yaw(self, angle: f32) -> V3 {
        let (s, c) = angle.sin_cos();
        v(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
    }
}

/// How the head is held at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    /// Forward nod in radians, about the top of the neck.
    pub nod: f32,
    /// How far the hair trails behind the nod (radians, added at the hair tips).
    pub hair_lag: f32,
    /// The whole head moves down by this much (virtual pixels), and toward the viewer by half.
    pub drop: f32,
    /// The body (neck and shoulders) moves down by this much.
    pub body: f32,
    /// Head turn in radians: positive looks toward the viewer's right.
    pub turn: f32,
    /// The clasped hands: 1.0 held in front of the chest, 0.0 dropped out of view.
    pub hands: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Material {
    Skin,
    Hair,
    Lens,
    Frame,
    Earring,
    Hand,
    Sleeve,
    Cuff,
}

// Where the head sits on the virtual screen, and the camera looking at it.
const SCREEN_X: f32 = VIRTUAL_W as f32 / 2.0;
const SCREEN_Y: f32 = 45.0;
const CAMERA_Z: f32 = 150.0;
/// Magnification: 1.0 draws the middle of the head at one unit per virtual pixel.
const ZOOM: f32 = 1.22;
/// The head nods about this point: the top of the neck, toward the back.
const PIVOT: V3 = v(0.0, -22.0, -5.0);
/// A sphere around everything that moves, to skip rays that miss.
const BOUND_CENTER: V3 = v(0.0, 6.0, 0.0);
const BOUND_RADIUS: f32 = 46.0;

/// One rendered pixel of the head: its color, and its material for outlining.
type Hit = Option<Rgb>;

/// Draw the head (and neck) in `pose` over `out`, with a dark outline around it.
pub fn render(out: &mut [Rgb], pose: Pose) {
    let w = VIRTUAL_W as usize;
    let mut hits: Vec<Hit> = vec![None; out.len()];
    // Rows are independent: share them out across the CPU cores.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    let rows_per = (VIRTUAL_H as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for (chunk, rows) in hits.chunks_mut(rows_per * w).enumerate() {
            scope.spawn(move || {
                for (i, hit) in rows.iter_mut().enumerate() {
                    let (x, y) = (i % w, chunk * rows_per + i / w);
                    *hit = trace(x as f32 + 0.5, y as f32 + 0.5, pose);
                }
            });
        }
    });

    for (i, hit) in hits.iter().enumerate() {
        match hit {
            Some(color) => out[i] = *color,
            None => {
                let (x, y) = (i % w, i / w);
                let near = [(0, -1), (0, 1), (-1, 0), (1, 0)].iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    (0..w as i32).contains(&nx)
                        && (0..VIRTUAL_H as i32).contains(&ny)
                        && hits[ny as usize * w + nx as usize].is_some()
                });
                if near {
                    out[i] = OUTLINE;
                }
            }
        }
    }
}

/// March the ray through screen pixel (sx, sy).
fn trace(sx: f32, sy: f32, pose: Pose) -> Hit {
    let origin = v(0.0, 0.0, CAMERA_Z);
    let dir = v(sx - SCREEN_X, SCREEN_Y - sy, -CAMERA_Z * ZOOM).norm();

    // Skip straight to the bounding sphere, or give up if the ray misses it.
    let oc = origin.sub(BOUND_CENTER);
    let b = oc.dot(dir);
    let disc = b * b - (oc.dot(oc) - BOUND_RADIUS * BOUND_RADIUS);
    let neck_ray = sy > SCREEN_Y + 20.0; // the neck reaches below the sphere
    if disc < 0.0 && !neck_ray {
        return None;
    }
    let mut t = if disc >= 0.0 {
        (-b - disc.sqrt()).max(0.0)
    } else {
        CAMERA_Z - 40.0
    };
    let t_max = CAMERA_Z + 60.0;

    for _ in 0..96 {
        let p = origin.add(dir.mul(t));
        let (d, material) = scene(p, pose);
        if d < 0.04 {
            return Some(shade(p, dir, material, pose));
        }
        // Bent hair and the cheap tapered capsules are not exact distances: step a bit short.
        t += d * 0.8;
        if t > t_max {
            break;
        }
    }
    None
}

/// The world: the posed head, plus the neck and hands, which move with the body.
fn scene(p: V3, pose: Pose) -> (f32, Material) {
    // The neck ends flat at the jacket's collar (drawn in 2D around it).
    let neck_bottom = -34.0 - pose.body;
    let neck_mid = (-20.0 + neck_bottom) / 2.0 - pose.body / 2.0;
    let neck = cylinder_y(p.sub(v(0.0, neck_mid, -4.0)), 9.8, 7.0);
    let mut best = head(to_head_space(p, pose), pose.hair_lag);
    if neck < best.0 {
        best = (neck, Material::Skin);
    }
    if pose.hands > 0.01 {
        // Held just under the chin, a little larger than modeled so they read clearly. Out of
        // view, they sink below the screen and back toward the body.
        let away = 1.0 - pose.hands;
        let place = v(0.0, HANDS_LIFT - pose.body - away * 50.0, -away * 8.0);
        let q = p
            .sub(place)
            .sub(HANDS_CENTER)
            .mul(1.0 / HANDS_SCALE)
            .add(HANDS_CENTER);
        let (d, m) = hands(q);
        if d * HANDS_SCALE < best.0 {
            best = (d * HANDS_SCALE, m);
        }
    }
    best
}

/// The hands are modeled around HANDS_CENTER, then raised by HANDS_LIFT and scaled up.
const HANDS_CENTER: V3 = v(0.0, -36.0, 13.0);
const HANDS_LIFT: f32 = 10.0;
const HANDS_SCALE: f32 = 1.15;

fn to_head_space(p: V3, pose: Pose) -> V3 {
    let shifted = p.sub(v(0.0, -pose.drop, pose.drop * 0.5));
    shifted
        .sub(PIVOT)
        .pitch(-pose.nod)
        .yaw(-pose.turn)
        .add(PIVOT)
}

/// Hands clasped in front of the chest, fingers interlaced, thumbs resting on top; leather sleeves
/// with white shirt cuffs, and a watch on the right wrist.
fn hands(p: V3) -> (f32, Material) {
    // Quick exit far from the hands: everything below is within this box.
    let far = (p.x.abs() - 32.0).max(p.y + 24.0).max(-(p.y + 66.0));
    if far > 6.0 {
        return (far, Material::Skin);
    }
    let mut skin = ellipsoid(p.sub(v(0.0, -37.5, 11.5)), v(10.5, 5.5, 6.0));
    // Interlaced fingers: left-hand fingers reach right, right-hand fingers reach left, in turns.
    for i in 0..4 {
        let y = -33.0 - i as f32 * 2.5;
        let left = capsule(p, v(-8.5, y, 15.5), v(5.5, y - 0.5, 18.2), 1.35);
        let right = capsule(p, v(8.5, y - 1.25, 15.5), v(-5.5, y - 1.75, 18.2), 1.35);
        skin = smin(skin, left.min(right), 1.0);
    }
    let thumbs = capsule(p, v(-3.2, -32.0, 15.0), v(-0.7, -27.5, 17.2), 1.5).min(capsule(
        p,
        v(3.2, -32.0, 15.0),
        v(0.7, -27.5, 17.2),
        1.5,
    ));
    skin = smin(skin, thumbs, 1.2);

    let side = p.x.signum();
    let sleeve = tapered(
        p,
        v(16.0 * side, -42.0, 8.0),
        v(30.0 * side, -66.0, 2.0),
        4.6,
        5.6,
    );
    let cuff = capsule(
        p,
        v(11.5 * side, -38.5, 9.8),
        v(14.6 * side, -41.8, 8.6),
        3.7,
    );
    let watch = capsule(p, v(12.0, -38.4, 10.6), v(13.2, -40.2, 10.3), 3.5);

    let mut best = (skin, Material::Hand);
    for (d, m) in [
        (sleeve, Material::Sleeve),
        (cuff, Material::Cuff),
        (watch, Material::Earring),
    ] {
        if d < best.0 {
            best = (d, m);
        }
    }
    best
}

/// The head in its own space: distance to the nearest surface, and what that surface is.
fn head(p: V3, hair_lag: f32) -> (f32, Material) {
    // Skin: skull, jaw, chin, brow, nose and ears, blended into one smooth shape.
    let mut skin = ellipsoid(p.sub(v(0.0, 4.0, -1.0)), v(16.5, 19.0, 18.0));
    skin = smin(
        skin,
        ellipsoid(p.sub(v(0.0, -8.0, 2.5)), v(13.5, 16.0, 14.5)),
        6.0,
    );
    skin = smin(
        skin,
        ellipsoid(p.sub(v(0.0, -21.0, 7.5)), v(6.5, 5.0, 6.5)),
        4.0,
    );
    skin = smin(
        skin,
        ellipsoid(p.sub(v(0.0, 4.5, 13.5)), v(12.5, 3.0, 4.0)),
        3.0,
    );
    skin = smin(
        skin,
        ellipsoid(p.sub(v(0.0, -5.0, 16.0)), v(2.4, 5.5, 3.4)),
        2.0,
    );
    let ear_x = 16.2 * p.x.signum();
    skin = smin(
        skin,
        ellipsoid(p.sub(v(ear_x, -3.0, -1.0)), v(2.4, 5.5, 4.0)),
        2.0,
    );

    let hair = hair(bend(p, hair_lag));

    // Round sunglasses: two lenses in thin rings, a bridge and the arms to the ears.
    let lens_x = 7.2 * p.x.signum();
    let lens_c = v(lens_x, 0.0, 17.0);
    let q = p.sub(lens_c);
    let lens = cylinder_z(q, 4.6, 0.5);
    let ring = {
        let flat = (q.x * q.x + q.y * q.y).sqrt() - 4.7;
        (flat * flat + q.z * q.z).sqrt() - 0.75
    };
    let bridge = capsule(p, v(-2.6, 0.8, 17.4), v(2.6, 0.8, 17.4), 0.6);
    let arm_x = p.x.signum();
    let arm = capsule(
        p,
        v(11.6 * arm_x, 0.6, 15.8),
        v(16.9 * arm_x, 1.0, 1.0),
        0.6,
    );
    let frame = ring.min(bridge).min(arm);

    // A small hoop hanging from the right ear.
    let earring = capsule(p, v(17.6, -8.5, -0.5), v(17.6, -12.5, -0.5), 0.9);

    let mut best = (skin, Material::Skin);
    for (d, m) in [
        (hair, Material::Hair),
        (lens, Material::Lens),
        (frame, Material::Frame),
        (earring, Material::Earring),
    ] {
        if d < best.0 {
            best = (d, m);
        }
    }
    best
}

/// The hair: a close cap above the hairline, and a swept-up quiff of spikes.
fn hair(p: V3) -> f32 {
    // Hairline: high over the forehead, lower toward the back of the head.
    let hairline = 6.0 + if p.z > 0.0 { 0.45 } else { 0.6 } * p.z;
    let cap = ellipsoid(p.sub(v(0.0, 9.5, -2.5)), v(17.6, 17.5, 18.8)).max(hairline - p.y);

    // The quiff: thick locks rising from the front hairline and sweeping up and back, tallest at
    // the front; shorter ones over the crown and sides follow the same direction.
    const LOCKS: [(V3, V3, f32); 12] = [
        (v(0.0, 16.0, 11.0), v(1.0, 34.0, -1.0), 7.0),
        (v(-6.0, 16.0, 9.5), v(-8.0, 32.0, -3.0), 6.5),
        (v(6.5, 16.0, 9.0), v(9.0, 31.5, -4.0), 6.5),
        (v(-11.0, 13.5, 5.0), v(-15.0, 27.0, -6.0), 5.5),
        (v(11.0, 13.5, 5.0), v(15.5, 26.5, -7.0), 5.5),
        (v(-2.0, 19.0, 1.0), v(-3.0, 31.0, -12.0), 6.0),
        (v(4.0, 19.0, 0.0), v(6.0, 30.0, -13.0), 6.0),
        (v(-7.0, 18.0, -6.0), v(-10.0, 27.0, -16.0), 5.0),
        (v(7.0, 18.0, -6.0), v(10.0, 27.0, -16.0), 5.0),
        (v(0.0, 15.0, -12.0), v(0.0, 22.0, -21.0), 5.0),
        (v(-14.0, 9.0, -2.0), v(-18.5, 18.0, -9.0), 4.0),
        (v(14.0, 9.0, -2.0), v(18.5, 18.0, -9.0), 4.0),
    ];
    let mut d = cap;
    for (base, tip, r) in LOCKS {
        d = smin(d, tapered(p, base, tip, r, 1.2), 5.0);
    }
    d
}

/// The hair trails the head: the higher a point, the more of `lag` it gets, tipping it forward
/// when the head comes up and back when it goes down.
fn bend(p: V3, lag: f32) -> V3 {
    let k = ((p.y - 10.0) / 28.0).clamp(0.0, 1.0);
    let pivot = v(0.0, 10.0, 0.0);
    p.sub(pivot).pitch(-lag * k * k).add(pivot)
}

// Colors: each material is a ramp from shadow to highlight.
const OUTLINE: Rgb = theme::logo::OUTLINE;
const SKIN: [Rgb; 4] = [
    [112, 66, 58],
    [170, 106, 88],
    [218, 154, 124],
    [244, 194, 160],
];
const STUBBLE: [Rgb; 4] = [
    [96, 86, 84],
    [138, 128, 122],
    [176, 168, 160],
    [206, 198, 190],
];
const MOUTH: [Rgb; 4] = [[70, 22, 26], [96, 34, 36], [122, 50, 50], [146, 66, 62]];
const HAIR: [Rgb; 4] = [
    [112, 100, 76],
    [172, 158, 112],
    [220, 208, 158],
    [246, 240, 204],
];
pub(crate) const LENS: [Rgb; 4] = [[150, 50, 10], [196, 70, 16], [255, 122, 26], [255, 176, 72]]; // the logo's
const FRAME: [Rgb; 4] = [[34, 28, 34], [70, 62, 62], [140, 130, 122], [210, 200, 190]];
const SILVER: [Rgb; 4] = [
    [90, 96, 106],
    [150, 158, 168],
    [204, 210, 218],
    [244, 246, 250],
];
const LEATHER: [Rgb; 4] = [[16, 14, 20], [30, 28, 38], [58, 58, 72], [96, 98, 116]];
pub(crate) const CUFF: [Rgb; 4] = [
    [150, 152, 160],
    [196, 198, 206],
    [228, 230, 236],
    [250, 250, 252],
];
const GLINT: Rgb = [255, 246, 228];
const RIM: Rgb = [70, 230, 220]; // teal back light, like the lasers behind

fn shade(p: V3, dir: V3, material: Material, pose: Pose) -> Rgb {
    let n = normal(p, pose);
    let light = v(-0.55, 0.6, 0.6).norm();
    let diffuse = n.dot(light).max(0.0);
    let level = 0.22 + diffuse * 0.85;
    let ramp_at = |ramp: &[Rgb; 4]| ramp[((level * 4.0) as usize).min(3)];

    // Face details live on the skin: stubble, a goatee, the mouth.
    let local = to_head_space(p, pose);
    let color = match material {
        Material::Skin => {
            let mouth_y = -15.0 + 0.035 * local.x * local.x;
            if local.z > 10.0 && local.x.abs() < 5.5 && (local.y - mouth_y).abs() < 0.7 {
                ramp_at(&MOUTH)
            } else if local.z > 4.0 && local.y < -12.3 && local.x.abs() < 9.5 && local.y > -27.0 {
                // Stubble is a dither of grey over skin, denser on the goatee and mustache.
                let goatee = local.x.abs() < 5.0 && (local.y < -16.5 || local.y > -14.2);
                let speck = hash(local) < if goatee { 0.85 } else { 0.45 };
                if speck {
                    ramp_at(&STUBBLE)
                } else {
                    ramp_at(&SKIN)
                }
            } else {
                ramp_at(&SKIN)
            }
        }
        Material::Hair => {
            // Strands: streaks running up and back along the locks, one shade darker.
            let streak = (local.x * 1.4 + local.z * 0.5 - local.y * 0.15).sin() > 0.55;
            let i = ((level * 4.0) as usize).min(3);
            HAIR[if streak { i.saturating_sub(1) } else { i }]
        }
        Material::Hand => ramp_at(&SKIN),
        Material::Sleeve => ramp_at(&LEATHER),
        Material::Cuff => ramp_at(&CUFF),
        Material::Frame => ramp_at(&FRAME),
        Material::Earring => ramp_at(&SILVER),
        Material::Lens => {
            let reflect = dir.sub(n.mul(2.0 * dir.dot(n)));
            if reflect.dot(light) > 0.93 {
                GLINT
            } else {
                ramp_at(&LENS)
            }
        }
    };

    // Teal rim light on edges turned to the right and back.
    let facing = (-dir.dot(n)).max(0.0);
    let rim = (1.0 - facing).powi(3) * (0.4 + 0.6 * n.x.max(0.0));
    if rim > 0.45 && material != Material::Lens {
        mix(color, RIM, 0.55)
    } else {
        color
    }
}

fn normal(p: V3, pose: Pose) -> V3 {
    const E: f32 = 0.08;
    let d = |q: V3| scene(q, pose).0;
    let k = [
        v(1.0, -1.0, -1.0),
        v(-1.0, -1.0, 1.0),
        v(-1.0, 1.0, -1.0),
        v(1.0, 1.0, 1.0),
    ];
    k.iter()
        .fold(v(0.0, 0.0, 0.0), |acc, &k| {
            acc.add(k.mul(d(p.add(k.mul(E)))))
        })
        .norm()
}

// Distance functions for the basic solids (after Inigo Quilez's catalogue).

fn ellipsoid(p: V3, r: V3) -> f32 {
    let k0 = v(p.x / r.x, p.y / r.y, p.z / r.z).len();
    let k1 = v(p.x / (r.x * r.x), p.y / (r.y * r.y), p.z / (r.z * r.z)).len();
    k0 * (k0 - 1.0) / k1.max(1e-6)
}

fn capsule(p: V3, a: V3, b: V3, r: f32) -> f32 {
    tapered(p, a, b, r, r)
}

/// A capsule whose radius goes from `ra` at `a` to `rb` at `b`.
fn tapered(p: V3, a: V3, b: V3, ra: f32, rb: f32) -> f32 {
    let (pa, ba) = (p.sub(a), b.sub(a));
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    pa.sub(ba.mul(h)).len() - (ra + (rb - ra) * h)
}

/// A disc of radius `r` and half-thickness `h`, facing the viewer.
/// An upright cylinder of radius `r` and half-height `h`, with flat ends.
fn cylinder_y(p: V3, r: f32, h: f32) -> f32 {
    cylinder_z(v(p.x, p.z, p.y), r, h)
}

fn cylinder_z(p: V3, r: f32, h: f32) -> f32 {
    let dx = (p.x * p.x + p.y * p.y).sqrt() - r;
    let dz = p.z.abs() - h;
    dx.max(dz).min(0.0) + (dx.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt()
}

/// Smooth minimum: joins two shapes with a fillet of size `k`.
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8)
}

/// A stable pseudo-random value in 0..1 for a point, so stubble specks stay put as the head moves.
fn hash(p: V3) -> f32 {
    let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
    let mut h =
        (x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ z.wrapping_mul(83492791)) as u32;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pose: Pose) -> Vec<Rgb> {
        let mut out = vec![[0, 0, 0]; (VIRTUAL_W * VIRTUAL_H) as usize];
        render(&mut out, pose);
        out
    }

    /// Average screen row of pixels in any of `colors`, and how many there are.
    fn rows_of(frame: &[Rgb], colors: &[Rgb]) -> (f32, usize) {
        let ys: Vec<usize> = (0..frame.len())
            .filter(|&i| colors.contains(&frame[i]))
            .map(|i| i / VIRTUAL_W as usize)
            .collect();
        (
            ys.iter().sum::<usize>() as f32 / ys.len().max(1) as f32,
            ys.len(),
        )
    }

    #[test]
    fn head_is_big_and_centered() {
        let f = frame(Pose::default());
        let drawn: Vec<usize> = (0..f.len()).filter(|&i| f[i] != [0, 0, 0]).collect();
        let top = drawn.iter().map(|i| i / VIRTUAL_W as usize).min().unwrap();
        let columns: Vec<usize> = drawn.iter().map(|i| i % VIRTUAL_W as usize).collect();
        let (left, right) = (
            *columns.iter().min().unwrap(),
            *columns.iter().max().unwrap(),
        );
        // From the hair near the top of the screen down past the bottom (the neck).
        assert!(top < 12, "top {top}");
        assert!(right - left > 34, "width {}", right - left);
        assert!(((left + right) as i32 / 2 - VIRTUAL_W as i32 / 2).abs() <= 3);
    }

    #[test]
    fn glasses_are_orange_and_at_eye_level() {
        let (row, count) = rows_of(&frame(Pose::default()), &LENS);
        assert!(count > 30, "{count} lens pixels");
        assert!((35.0..52.0).contains(&row), "lenses at row {row}");
    }

    #[test]
    fn nodding_brings_the_face_down_and_shows_the_hair() {
        let still = frame(Pose::default());
        let nod = frame(Pose {
            nod: 0.45,
            drop: 3.0,
            ..Pose::default()
        });
        let (lens_still, _) = rows_of(&still, &LENS);
        let (lens_nod, _) = rows_of(&nod, &LENS);
        assert!(
            lens_nod > lens_still + 3.0,
            "lenses {lens_still} -> {lens_nod}"
        );
        // Tipped toward the viewer, more of the top of the head (hair) is in view.
        let (_, hair_still) = rows_of(&still, &HAIR);
        let (_, hair_nod) = rows_of(&nod, &HAIR);
        assert!(
            hair_nod > hair_still,
            "hair pixels {hair_still} -> {hair_nod}"
        );
    }

    #[test]
    fn hair_trails_the_head() {
        let straight = frame(Pose {
            nod: 0.2,
            ..Pose::default()
        });
        let trailing = frame(Pose {
            nod: 0.2,
            hair_lag: -0.3,
            ..Pose::default()
        });
        assert_ne!(straight, trailing);
    }
}
