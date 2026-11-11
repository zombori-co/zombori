//! What the gate shows at each moment, drawn on the small virtual screen: the same 3D man
//! throughout, in front of laser beams. He asks the question with his hands clasped, listens,
//! thinks, shakes his head at a wrong answer, and on the right one drops his hands and headbangs.
//! No hardware involved, so this runs (and is tested) anywhere.

use theme::{BACKGROUND, Rgb, VIRTUAL_H, VIRTUAL_W, ease, fade};

use crate::beat::Beat;
use crate::head3d::{self, Pose};

const ORANGE: Rgb = [255, 122, 26]; // the logo's lens color

/// The headbang at full strength: how far the head nods forward at the bottom of each beat (and
/// back at the top), how far it drops, how far the body dips. It takes RAMP_S to build up to it.
const NOD_DOWN: f32 = 0.55;
const NOD_UP: f32 = -0.1;
const DROP_PX: f32 = 4.0;
const BODY_PX: f32 = 2.0;
const RAMP_S: f32 = 3.0;
/// The hair follows the head this fraction of a beat late.
const HAIR_DELAY: f32 = 0.12;
/// On the right answer the clasped hands drop out of view over this long.
const HANDS_DROP_S: f32 = 0.8;
/// The first time the man appears, he fades in from black over this long.
const FADE_IN_S: f32 = 0.6;
/// Screen row where the jacket's collar sits when the body is up.
const COLLAR_Y: i32 = 85;

const OUTLINE: Rgb = theme::logo::OUTLINE;
const LEATHER: Rgb = [24, 22, 30];
const LEATHER_LIGHT: Rgb = [70, 70, 86];
const CHEST: Rgb = [206, 142, 114];
const SILVER: Rgb = [200, 206, 214];
const LASER_TEAL: Rgb = [60, 220, 214];
const LASER_RED: Rgb = [226, 54, 70];
const RECORDING: Rgb = [230, 40, 50];
const WOOD_A: Rgb = [58, 34, 22];
const WOOD_B: Rgb = [66, 40, 26];
const WOOD_GAP: Rgb = [30, 16, 12];
const WOOD_GRAIN: Rgb = [44, 24, 16];
const WOOD_BEAM: Rgb = [40, 22, 14];
const LANTERN: Rgb = [255, 176, 82];

/// One moment of the gate. `t` is seconds since that stage started.
#[derive(Clone, Copy, Debug)]
pub enum View {
    /// The question is playing; `first` the first time it is asked (the man fades in).
    Asking { t: f32, first: bool },
    /// Recording the answer; `level` is the microphone level, 0.0..1.0.
    Listening { t: f32, level: f32 },
    /// Recognizing what was said.
    Thinking { t: f32 },
    /// That was not it.
    Wrong { t: f32 },
    /// Correct: the hands drop, and the headbang starts with the music `music` seconds in (after
    /// a spoken intro, for example).
    Celebrating { t: f32, beat: Beat, music: f32 },
    /// A room was chosen: he bows toward it and the picture fades out before the app opens.
    Opening { t: f32 },
}

/// A wrong answer's head shake lasts this long before asking again.
pub const WRONG_S: f32 = 1.2;
/// The bow and fade before an app opens.
pub const OPENING_S: f32 = 1.0;

/// What is behind the man.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Backdrop {
    /// The gate: laser beams in the dark.
    #[default]
    Lasers,
    /// The tavern: a warm wooden wall lit by two lanterns.
    Tavern,
}

#[derive(Default)]
pub struct Scene {
    backdrop: Backdrop,
}

impl Scene {
    /// The gate's scene, with lasers behind him.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_backdrop(backdrop: Backdrop) -> Self {
        Self { backdrop }
    }

    pub fn render(&self, view: View, out: &mut [Rgb]) {
        let clasped = Pose {
            hands: 1.0,
            ..Pose::default()
        };
        // `lasers_at` is how bright the backdrop's lights are (lasers, or the tavern's lanterns).
        let (pose, lasers_at, red, fade_in) = match view {
            // Asking: calm, a slight breathing nod.
            View::Asking { t, first } => {
                let nod = 0.03 * (t * 1.7).sin();
                let fade_in = if first { ease(t / FADE_IN_S) } else { 1.0 };
                (Pose { nod, ..clasped }, 0.2, 0.0, fade_in)
            }
            // Listening: leaning in a little; the lasers glow with the voice.
            View::Listening { t, level } => {
                let nod = 0.08 * ease(t / 0.4);
                let lasers = 0.15 + 0.75 * level.clamp(0.0, 1.0);
                (Pose { nod, ..clasped }, lasers, 0.0, 1.0)
            }
            // Thinking: a glance to the side.
            View::Thinking { t } => {
                let turn = 0.24 * ease(t / 0.4);
                (
                    Pose {
                        turn,
                        nod: 0.05,
                        ..clasped
                    },
                    0.2,
                    0.0,
                    1.0,
                )
            }
            // Wrong: a "no" shake that dies down, and a red flash.
            View::Wrong { t } => {
                let turn = 0.4 * (t * 13.0).sin() * (-t * 2.8).exp();
                (Pose { turn, ..clasped }, 0.6, (-t * 2.0).exp(), 1.0)
            }
            // Correct: the hands drop, then the headbang starts with the music.
            View::Celebrating { t, beat, music } => {
                let beats = beat.at(t);
                let amplitude = ease((t - music) / RAMP_S);
                let pose = Pose {
                    hands: 1.0 - ease(t / HANDS_DROP_S),
                    ..pose_at(beats, amplitude)
                };
                // The lasers flare on every hit.
                (pose, 0.25 + 0.6 * bang(beats) * amplitude, 0.0, 1.0)
            }
            // Opening: a small bow, then everything fades to black.
            View::Opening { t } => {
                let nod = 0.18 * ease(t / 0.4);
                (Pose { nod, ..clasped }, 0.3, 0.0, 1.0 - ease(t / OPENING_S))
            }
        };

        out.fill(BACKGROUND);
        let t = match view {
            View::Asking { t, .. }
            | View::Listening { t, .. }
            | View::Thinking { t }
            | View::Wrong { t }
            | View::Celebrating { t, .. }
            | View::Opening { t } => t,
        };
        match self.backdrop {
            Backdrop::Lasers => lasers(out, t, lasers_at, red),
            Backdrop::Tavern => tavern_wall(out, t, lasers_at, red),
        }
        jacket(out, COLLAR_Y + pose.body.round() as i32);
        head3d::render(out, pose);
        match view {
            View::Listening { t, .. } => recording_dot(out, t),
            View::Thinking { t } => dots(out, t),
            _ => {}
        }
        if fade_in < 1.0 {
            for p in out.iter_mut() {
                *p = mix(BACKGROUND, *p, fade_in);
            }
        }
    }
}

/// The headbang over one beat, 0.0 (head up) to 1.0 (bottom of the nod). The head hits the
/// bottom exactly on the beat, rebounds fast and floats up, then accelerates down into the next
/// hit.
pub fn bang(beats: f32) -> f32 {
    const UP: f32 = 0.6; // share of the beat spent going up
    let phase = beats.rem_euclid(1.0);
    if phase < UP {
        (1.0 - phase / UP).powi(2)
    } else {
        ((phase - UP) / (1.0 - UP)).powi(2)
    }
}

/// The head's pose at `beats` into the music, at `amplitude` (0.0 still, 1.0 full headbang).
fn pose_at(beats: f32, amplitude: f32) -> Pose {
    let nod = |b: f32| amplitude * (NOD_UP + (NOD_DOWN - NOD_UP) * bang(b));
    let hit = bang(beats) * amplitude;
    Pose {
        nod: nod(beats),
        // The hair is where the head was a moment ago: it keeps falling as the head rebounds.
        hair_lag: (nod(beats - HAIR_DELAY) - nod(beats)) * 0.9,
        drop: DROP_PX * hit,
        body: BODY_PX * hit,
        ..Pose::default()
    }
}

/// Thin laser beams crossing the dark behind the man, drifting slowly; `red` turns them all red.
fn lasers(out: &mut [Rgb], t: f32, strength: f32, red: f32) {
    const BEAMS: [(f32, f32, f32, f32, bool); 7] = [
        (-10.0, 8.0, 170.0, 70.0, true),
        (-10.0, 90.0, 170.0, 20.0, true),
        (30.0, -10.0, 120.0, 110.0, true),
        (150.0, -10.0, 10.0, 110.0, false),
        (-10.0, 40.0, 170.0, 55.0, true),
        (100.0, -10.0, 170.0, 60.0, true),
        (-10.0, 64.0, 60.0, -10.0, false),
    ];
    for (i, &(x0, y0, x1, y1, teal)) in BEAMS.iter().enumerate() {
        let drift = (t * 0.3 + i as f32 * 1.7).sin() * 6.0;
        let color = mix(if teal { LASER_TEAL } else { LASER_RED }, LASER_RED, red);
        line(out, x0, y0 + drift, x1, y1 - drift, color, strength);
    }
}

/// The tavern's back wall: dark wooden planks under a beam, warmed by two flickering lanterns
/// whose glow follows `glow` (the voice, while listening); `red` flashes it red.
fn tavern_wall(out: &mut [Rgb], t: f32, glow: f32, red: f32) {
    const PLANK: i32 = 11;
    const BEAM_Y: i32 = 16;
    const LANTERNS: [(f32, f32); 2] = [(22.0, 34.0), (138.0, 34.0)];
    let flicker = 0.9 + 0.1 * (t * 9.0).sin() * (t * 5.3).cos();
    for y in 0..VIRTUAL_H as i32 {
        for x in 0..VIRTUAL_W as i32 {
            let plank = x / PLANK;
            let base = if (BEAM_Y - 4..BEAM_Y).contains(&y) {
                WOOD_BEAM
            } else if x % PLANK == 0 {
                WOOD_GAP
            } else if (x * 7 + plank * 13 + y / 9 * 3) % 23 == 0 {
                WOOD_GRAIN // a knot or a streak of grain
            } else if plank % 2 == 0 {
                WOOD_A
            } else {
                WOOD_B
            };
            let warmth: f32 = LANTERNS
                .iter()
                .map(|&(lx, ly)| {
                    let d2 = (x as f32 - lx).powi(2) + (y as f32 - ly).powi(2) * 1.4;
                    (-d2 / 900.0).exp()
                })
                .sum();
            let k = (warmth * (0.35 + 0.65 * glow) * flicker).min(1.0);
            let lit = mix(base, LANTERN, k * 0.55);
            out[(y as u32 * VIRTUAL_W + x as u32) as usize] = mix(lit, LASER_RED, red * 0.35);
        }
    }
    // The lanterns themselves: a bright flame in a dark frame.
    for &(lx, ly) in &LANTERNS {
        let (lx, ly) = (lx as i32, ly as i32);
        for dy in -3i32..=3 {
            for dx in -2i32..=2 {
                let edge = dx.abs() == 2 || dy.abs() == 3;
                put(
                    out,
                    lx + dx,
                    ly + dy,
                    if edge {
                        OUTLINE
                    } else {
                        mix(LANTERN, [255, 250, 220], flicker - 0.8)
                    },
                );
            }
        }
    }
}

/// A 1-pixel line blended over what is there.
fn line(out: &mut [Rgb], x0: f32, y0: f32, x1: f32, y1: f32, color: Rgb, k: f32) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil() as usize;
    for i in 0..=steps {
        let s = i as f32 / steps.max(1) as f32;
        let (x, y) = (
            (x0 + (x1 - x0) * s).round() as i32,
            (y0 + (y1 - y0) * s).round() as i32,
        );
        if (0..VIRTUAL_W as i32).contains(&x) && (0..VIRTUAL_H as i32).contains(&y) {
            let i = (y as u32 * VIRTUAL_W + x as u32) as usize;
            out[i] = mix(out[i], color, k);
        }
    }
}

/// A black leather jacket from the collar down: wide shoulders, lapels around an open V with the
/// chest and two necklaces showing. Drawn before the 3D man, so his neck and hands sit on it.
fn jacket(out: &mut [Rgb], top: i32) {
    let cx = VIRTUAL_W as i32 / 2;
    for y in top..VIRTUAL_H as i32 {
        let depth = y - top;
        let half = (13 + depth * 6).min(76);
        let open = (10 - depth / 3).max(0); // half-width of the V
        for x in (cx - half)..=(cx + half) {
            let off = (x - cx).abs();
            let color = if off == half || depth == 0 && off > open {
                OUTLINE
            } else if off < open {
                CHEST
            } else if off <= open + 2 || (off >= half - 3 && depth > 2) {
                LEATHER_LIGHT // lapel edges and the shoulders catching the light
            } else {
                LEATHER
            };
            put(out, x, y, color);
        }
    }
    // Necklaces across the open V: a bead string and a fine chain with a pendant.
    for x in (cx - 12)..=(cx + 12) {
        let dx = (x - cx) as f32;
        let beads = top + 2 + (dx * dx / 40.0) as i32;
        if (x - cx) % 2 == 0 {
            put(out, x, beads, OUTLINE);
        }
        let chain = top + 5 + (dx * dx / 22.0) as i32;
        if (x - cx).abs() < 10 && (x - cx) % 3 == 0 {
            put(out, x, chain, SILVER);
        }
    }
    for dy in 0..3 {
        put(out, cx, top + 6 + dy, SILVER);
    }
}

/// A pulsing red dot in the corner: recording.
fn recording_dot(out: &mut [Rgb], t: f32) {
    let k = 0.55 + 0.45 * (t * 4.0).sin().abs();
    for (dx, dy) in [
        (1, 0),
        (2, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (0, 2),
        (1, 2),
        (2, 2),
        (3, 2),
        (1, 3),
        (2, 3),
    ] {
        put(out, 5 + dx, 5 + dy, fade(RECORDING, k));
    }
}

/// Three dots in the corner lighting up in turn: thinking.
fn dots(out: &mut [Rgb], t: f32) {
    for i in 0..3 {
        let phase = (t * 2.5 - i as f32 * 0.25).rem_euclid(1.0);
        let k = 0.3 + 0.7 * (1.0 - phase).powi(2);
        for dy in 0..2 {
            for dx in 0..2 {
                put(out, 5 + i * 4 + dx, 6 + dy, fade(ORANGE, k));
            }
        }
    }
}

fn put(out: &mut [Rgb], x: i32, y: i32, c: Rgb) {
    if (0..VIRTUAL_W as i32).contains(&x) && (0..VIRTUAL_H as i32).contains(&y) {
        out[(y as u32 * VIRTUAL_W + x as u32) as usize] = c;
    }
}

fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEAT: Beat = Beat {
        bpm: 120.0,
        first: 0.0,
    };

    fn frame(view: View) -> Vec<Rgb> {
        let mut out = vec![[0; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
        Scene::new().render(view, &mut out);
        out
    }

    fn count_any(frame: &[Rgb], colors: &[Rgb]) -> usize {
        frame.iter().filter(|p| colors.contains(p)).count()
    }

    /// Average screen position of the lenses (any shade: a nodded or turned lens is darker).
    fn lenses(f: &[Rgb]) -> (f32, f32) {
        let at: Vec<usize> = (0..f.len())
            .filter(|&i| head3d::LENS.contains(&f[i]))
            .collect();
        assert!(at.len() > 10, "lenses visible");
        let n = at.len() as f32;
        let x = at.iter().map(|i| i % VIRTUAL_W as usize).sum::<usize>() as f32 / n;
        let y = at.iter().map(|i| i / VIRTUAL_W as usize).sum::<usize>() as f32 / n;
        (x, y)
    }

    fn lens_row(t: f32, music: f32) -> f32 {
        lenses(&frame(View::Celebrating {
            t,
            beat: BEAT,
            music,
        }))
        .1
    }

    #[test]
    fn hands_are_clasped_until_the_right_answer() {
        let cuffs = |f: &[Rgb]| count_any(f, &head3d::CUFF);
        assert!(
            cuffs(&frame(View::Asking {
                t: 1.0,
                first: false
            })) > 10
        );
        assert!(cuffs(&frame(View::Listening { t: 1.0, level: 0.5 })) > 10);
        assert!(cuffs(&frame(View::Wrong { t: 0.3 })) > 10);
        // Gone once they have dropped, a moment into the celebration.
        let celebrating = frame(View::Celebrating {
            t: 1.5,
            beat: BEAT,
            music: 4.0,
        });
        assert_eq!(cuffs(&celebrating), 0);
    }

    #[test]
    fn lasers_glow_with_the_voice() {
        // Sum the brightness of the left edge, where only the lasers are.
        let glow = |level| -> u32 {
            let f = frame(View::Listening { t: 1.0, level });
            (0..VIRTUAL_H as usize)
                .flat_map(|y| (0..20).map(move |x| y * VIRTUAL_W as usize + x))
                .map(|i| f[i].iter().map(|&c| u32::from(c)).sum::<u32>())
                .sum()
        };
        assert!(glow(1.0) > glow(0.0) + 2000);
    }

    #[test]
    fn recording_dot_shows_only_while_listening() {
        let red = |f: &[Rgb]| (0..12).any(|i| f[i * VIRTUAL_W as usize + 7][0] > 150);
        assert!(red(&frame(View::Listening { t: 0.4, level: 0.0 })));
        assert!(!red(&frame(View::Asking {
            t: 0.4,
            first: false
        })));
    }

    #[test]
    fn wrong_answer_shakes_the_head_and_settles() {
        let (rest_x, _) = lenses(&frame(View::Asking {
            t: 0.0,
            first: false,
        }));
        let (shake_x, _) = lenses(&frame(View::Wrong { t: 0.12 }));
        let (settled_x, _) = lenses(&frame(View::Wrong { t: WRONG_S }));
        assert!(
            (shake_x - rest_x).abs() > 2.0,
            "shake {shake_x} vs rest {rest_x}"
        );
        assert!(
            (settled_x - rest_x).abs() < 1.0,
            "settled {settled_x} vs rest {rest_x}"
        );
    }

    #[test]
    fn tavern_is_warm_wood() {
        let mut out = vec![[0; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
        let view = View::Asking {
            t: 1.0,
            first: false,
        };
        Scene::with_backdrop(Backdrop::Tavern).render(view, &mut out);
        // The top-left corner is wall: brown, red above blue, and not the gate's dark background.
        let [r, g, b] = out[(4 * VIRTUAL_W + 4) as usize];
        assert!(r > g && g > b && r > 40, "wall color {:?}", [r, g, b]);
        // The man is still there in front of it.
        assert!(out.iter().any(|p| head3d::LENS.contains(p)));
    }

    #[test]
    fn opening_bows_and_fades_to_black() {
        let start = frame(View::Opening { t: 0.0 });
        assert!(start.iter().any(|&p| p != BACKGROUND));
        let end = frame(View::Opening { t: OPENING_S });
        assert!(end.iter().all(|&p| p == BACKGROUND));
    }

    #[test]
    fn first_question_fades_in() {
        let first = frame(View::Asking {
            t: 0.0,
            first: true,
        });
        assert!(first.iter().all(|&p| p == BACKGROUND));
        let again = frame(View::Asking {
            t: 0.0,
            first: false,
        });
        assert!(again.iter().any(|&p| p != BACKGROUND));
    }

    #[test]
    fn bang_hits_bottom_on_the_beat() {
        assert_eq!(bang(3.0), 1.0);
        assert!(bang(3.6) < 1e-6); // the top of the nod
        assert!(bang(3.97) > 0.85 && bang(3.05) > 0.8); // close to the hit on both sides
        // No jumps: one beat sampled finely changes smoothly.
        for i in 0..400 {
            let (a, b) = (bang(i as f32 / 400.0), bang((i + 1) as f32 / 400.0));
            assert!((a - b).abs() < 0.03, "jump at {i}");
        }
    }

    #[test]
    fn head_bangs_down_on_the_beat() {
        // 120 BPM: beats every 0.5 s. Past the ramp, the face is far lower on the beat (8.0 s)
        // than at the top of the nod (8.3 s, 60 % into the beat).
        let (hit, top) = (lens_row(8.0, 0.0), lens_row(8.3, 0.0));
        assert!(
            hit > top + 4.0,
            "lenses at row {hit} on the beat, {top} at the top"
        );
    }

    #[test]
    fn head_waits_for_the_music_and_builds_up() {
        // Music from 4.6 s, as after a spoken "yes": the head holds still before it...
        assert_eq!(lens_row(1.0, 4.6), lens_row(1.3, 4.6));
        // ...then nods a little at first and fully later.
        let early = lens_row(5.0, 4.6) - lens_row(5.3, 4.6);
        let full = lens_row(9.0, 4.6) - lens_row(9.3, 4.6);
        assert!(early < full / 2.0, "early nod {early} rows, full {full}");
    }

    #[test]
    fn neutral_pose_before_the_music() {
        assert_eq!(pose_at(7.3, 0.0), Pose::default());
    }

    #[test]
    fn every_view_renders_without_panicking() {
        for i in 0..120 {
            let t = i as f32 * 0.08;
            for view in [
                View::Asking { t, first: true },
                View::Listening {
                    t,
                    level: (t * 0.3).fract(),
                },
                View::Thinking { t },
                View::Wrong { t },
                View::Celebrating {
                    t,
                    beat: BEAT,
                    music: 2.0,
                },
            ] {
                frame(view);
            }
        }
    }
}
