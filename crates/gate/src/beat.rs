//! Finding the beat of a piece of music, so the head moves in time with whatever track is
//! installed. Classic onset autocorrelation: find where the sound gets suddenly louder, find the
//! spacing at which those moments repeat best (the tempo), then where in that spacing they fall
//! (the phase). No hardware involved, so this runs (and is tested) anywhere.

/// Analysis step: 10 ms.
const HOP_S: f32 = 0.01;
const MIN_BPM: f32 = 70.0;
const MAX_BPM: f32 = 180.0;
/// Used when there is nothing to analyze (no music, or silence).
pub const FALLBACK: Beat = Beat {
    bpm: 120.0,
    first: 0.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beat {
    pub bpm: f32,
    /// Time of the first beat, in seconds.
    pub first: f32,
}

impl Beat {
    /// Beats elapsed at `t` seconds: 2.0 is the third beat, 2.5 halfway to the fourth.
    pub fn at(&self, t: f32) -> f32 {
        (t - self.first) * self.bpm / 60.0
    }
}

/// Estimate the beat of mono samples at `rate`.
pub fn detect(samples: &[f32], rate: u32) -> Beat {
    let onsets = onset_strength(samples, rate);
    let min_lag = (60.0 / MAX_BPM / HOP_S).floor() as usize;
    let max_lag = (60.0 / MIN_BPM / HOP_S).ceil() as usize;
    if onsets.len() < max_lag * 4 || onsets.iter().all(|&o| o == 0.0) {
        return FALLBACK;
    }

    // Tempo: the lag at which the onset curve best matches a shifted copy of itself. A mild
    // preference for tempos near 120 BPM settles half/double-tempo ties toward the usual one.
    let score = |lag: usize| -> f32 {
        let n = onsets.len() - lag;
        let sum: f32 = (0..n).map(|i| onsets[i] * onsets[i + lag]).sum();
        let bpm = 60.0 / (lag as f32 * HOP_S);
        let preference = (-(bpm / 120.0).log2().powi(2) / 2.0).exp();
        sum / n as f32 * (0.5 + 0.5 * preference)
    };
    let scores: Vec<f32> = (min_lag..=max_lag).map(score).collect();
    let best = (0..scores.len())
        .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
        .unwrap_or(0);
    // Refine between whole lags with a parabola through the peak and its neighbors.
    let shift = match (best.checked_sub(1), scores.get(best + 1)) {
        (Some(l), Some(&right)) => {
            let (left, mid) = (scores[l], scores[best]);
            let curve = left - 2.0 * mid + right;
            if curve < 0.0 {
                0.5 * (left - right) / curve
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    let period = (min_lag + best) as f32 + shift.clamp(-0.5, 0.5);

    // Phase: the offset within one period where the onsets line up best.
    let phase = (0..period.ceil() as usize)
        .max_by(|&a, &b| {
            let sum = |k: usize| -> f32 {
                (0..)
                    .map(|m| (k as f32 + m as f32 * period).round() as usize)
                    .take_while(|&i| i < onsets.len())
                    .map(|i| onsets[i])
                    .sum()
            };
            sum(a).total_cmp(&sum(b))
        })
        .unwrap_or(0);

    Beat {
        bpm: 60.0 / (period * HOP_S),
        first: phase as f32 * HOP_S,
    }
}

/// Where the music starts, in seconds. A clip may open with something spoken and a pause before
/// the music comes in; the music then starts at the end of the longest pause in the first
/// `INTRO_S` seconds that follows some sound. Without such a pause, it starts with the first sound.
pub fn music_start(samples: &[f32], rate: u32) -> f32 {
    /// Loudness steps of 100 ms.
    const STEP_S: f32 = 0.1;
    /// A pause is this much quieter than the clip's typical loudness...
    const QUIET_DB: f32 = 15.0;
    /// ...for at least this long (shorter dips happen between words and in quiet music).
    const MIN_PAUSE_STEPS: usize = 6;
    const INTRO_S: f32 = 15.0;

    let step = ((rate as f32 * STEP_S) as usize).max(1);
    let loudness: Vec<f32> = samples
        .chunks(step)
        .map(|c| 10.0 * (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32 + 1e-12).log10())
        .collect();
    if loudness.is_empty() {
        return 0.0;
    }
    let mut sorted = loudness.clone();
    sorted.sort_by(f32::total_cmp);
    let typical = sorted[sorted.len() / 2];
    let quiet: Vec<bool> = loudness.iter().map(|&db| db < typical - QUIET_DB).collect();

    let intro = quiet.len().min((INTRO_S / STEP_S) as usize);
    let first_sound = quiet.iter().position(|&q| !q).unwrap_or(0);
    let mut longest: Option<(usize, usize)> = None; // (length, end) of the longest pause
    let mut run = 0;
    for (i, &q) in quiet.iter().enumerate().take(intro).skip(first_sound) {
        if q {
            run += 1;
            continue;
        }
        if run >= MIN_PAUSE_STEPS && longest.is_none_or(|(len, _)| run > len) {
            longest = Some((run, i));
        }
        run = 0;
    }
    let start = longest.map_or(first_sound, |(_, end)| end);
    start as f32 * STEP_S
}

/// How suddenly the loudness rises in each 10 ms step (rises only; falls count as zero).
fn onset_strength(samples: &[f32], rate: u32) -> Vec<f32> {
    let hop = ((rate as f32 * HOP_S) as usize).max(1);
    let loudness: Vec<f32> = samples
        .chunks(hop)
        .map(|chunk| (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32 + 1e-9).ln())
        .collect();
    let mut onsets = vec![0.0];
    onsets.extend(loudness.windows(2).map(|w| (w[1] - w[0]).max(0.0)));
    onsets
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16000;

    /// A drum-like click track: a short decaying noise burst on every beat, softer off-beat hats.
    fn clicks(bpm: f32, first: f32, seconds: f32, hats: bool) -> Vec<f32> {
        let mut out = vec![0.0f32; (seconds * RATE as f32) as usize];
        let mut seed = 7u32;
        let mut noise = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 16) as f32 / 65535.0 - 0.5
        };
        let beat = 60.0 / bpm;
        let mut hit = |at: f32, amp: f32, decay: f32| {
            let start = (at * RATE as f32) as usize;
            for i in 0..(RATE as usize / 10) {
                if let Some(s) = out.get_mut(start + i) {
                    *s += noise() * amp * (-(i as f32) / (RATE as f32 * decay)).exp();
                }
            }
        };
        let mut t = first;
        while t < seconds {
            hit(t, 0.8, 0.03);
            if hats {
                hit(t + beat / 2.0, 0.25, 0.01);
            }
            t += beat;
        }
        // A quiet bed of noise, like the rest of a mix.
        for s in &mut out {
            *s += noise() * 0.01;
        }
        out
    }

    fn assert_beat(found: Beat, bpm: f32, first: f32) {
        assert!(
            (found.bpm - bpm).abs() < 2.0,
            "found {found:?}, expected {bpm} BPM"
        );
        let period = 60.0 / bpm;
        let off = (found.first - first).rem_euclid(period);
        let off = off.min(period - off);
        assert!(
            off < 0.03,
            "found {found:?}, expected first beat at {first} s"
        );
    }

    #[test]
    fn finds_tempo_and_phase() {
        assert_beat(detect(&clicks(120.0, 0.25, 10.0, false), RATE), 120.0, 0.25);
        assert_beat(detect(&clicks(96.0, 0.4, 10.0, false), RATE), 96.0, 0.4);
        assert_beat(detect(&clicks(140.0, 0.1, 10.0, false), RATE), 140.0, 0.1);
    }

    #[test]
    fn off_beat_hats_do_not_double_the_tempo() {
        assert_beat(detect(&clicks(128.0, 0.2, 10.0, true), RATE), 128.0, 0.2);
    }

    #[test]
    fn finds_the_placeholder_track_tempo() {
        // assets/sounds/celebrate.py writes a 126 BPM loop with its first beat at 0 s.
        let wav =
            audio::wav::parse(include_bytes!("../../../assets/sounds/celebrate.wav")).unwrap();
        assert_beat(detect(&wav.mono_f32(), wav.rate), 126.0, 0.0);
    }

    #[test]
    fn music_starts_after_a_spoken_intro() {
        // 0.5 s silence, 2 s of "speech" (with a short gap between words), a 1.2 s pause, music.
        let mut clip = vec![0.0f32; RATE as usize / 2];
        let mut seed = 3u32;
        let mut noise = |amp: f32| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 16) as f32 / 65535.0 - 0.5) * amp
        };
        for i in 0..(2 * RATE) as usize {
            let between_words = (RATE as usize..RATE as usize * 13 / 10).contains(&i);
            clip.push(if between_words { 0.0 } else { noise(0.3) });
        }
        clip.extend((0..(RATE as usize * 12 / 10)).map(|_| noise(0.002)));
        let music_at = clip.len() as f32 / RATE as f32; // 3.7 s
        // Real music is dense: drums over a sustained bassline.
        let bass =
            (0..).map(|i| (i as f32 / RATE as f32 * 110.0 * std::f32::consts::TAU).sin() * 0.2);
        clip.extend(
            clicks(120.0, 0.0, 10.0, true)
                .iter()
                .zip(bass)
                .map(|(c, b)| c + b),
        );
        let start = music_start(&clip, RATE);
        assert!(
            (start - music_at).abs() <= 0.15,
            "start {start}, music at {music_at}"
        );
    }

    #[test]
    fn music_without_intro_starts_at_once() {
        let wav =
            audio::wav::parse(include_bytes!("../../../assets/sounds/celebrate.wav")).unwrap();
        assert!(music_start(&wav.mono_f32(), wav.rate) < 0.15);
        assert_eq!(music_start(&[], RATE), 0.0);
    }

    #[test]
    fn silence_falls_back() {
        assert_eq!(detect(&vec![0.0; RATE as usize * 5], RATE), FALLBACK);
        assert_eq!(detect(&[], RATE), FALLBACK);
    }

    #[test]
    fn counts_beats() {
        let beat = Beat {
            bpm: 120.0,
            first: 0.5,
        };
        assert_eq!(beat.at(0.5), 0.0);
        assert_eq!(beat.at(1.5), 2.0);
    }
}
