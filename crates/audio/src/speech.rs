//! Finding where someone speaks in a microphone stream: wait for the voice to start, then stop
//! once it has been quiet for a moment. Works on loudness alone, which is enough for a short
//! spoken answer. No hardware involved, so this runs (and is tested) anywhere.

/// Length of one analysis frame. Feed the detector frames of exactly `frame_len()` samples.
const FRAME_MS: usize = 20;
/// The first frames only measure the background noise.
const CALIBRATION_FRAMES: usize = 10;
/// Loudness (RMS of 16-bit samples) that always counts as quiet, even in a silent room.
const MIN_SPEECH_RMS: f32 = 400.0;
/// Speech must be this many times louder than the background noise.
const NOISE_MARGIN: f32 = 3.0;
/// Frames in a row above the threshold before it counts as speech, so a click doesn't.
const START_FRAMES: usize = 4;
/// Quiet frames in a row that end the answer (0.7 s).
const END_FRAMES: usize = 35;
/// Audio kept before the detected start and after the end, so soft first and last sounds stay.
const PAD_FRAMES: usize = 10;
/// Give up waiting for speech after 8 s, and cut an answer off after 6 s.
const WAIT_FRAMES: usize = 8000 / FRAME_MS;
const MAX_SPEECH_FRAMES: usize = 6000 / FRAME_MS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Waiting,
    Speaking,
    /// The answer is in samples `start..end` of everything fed so far.
    Done {
        start: usize,
        end: usize,
    },
    NoSpeech,
}

pub struct Detector {
    frame_len: usize,
    frames: usize,
    noise: f32,
    loud_run: usize,
    quiet_run: usize,
    speech_start: Option<usize>,
    state: State,
}

impl Detector {
    pub fn new(rate: u32) -> Self {
        Self {
            frame_len: rate as usize * FRAME_MS / 1000,
            frames: 0,
            noise: 0.0,
            loud_run: 0,
            quiet_run: 0,
            speech_start: None,
            state: State::Waiting,
        }
    }

    pub fn frame_len(&self) -> usize {
        self.frame_len
    }

    /// Analyze the next frame and say where things stand. Once `Done` or `NoSpeech`, it stays so.
    pub fn feed(&mut self, frame: &[i16]) -> State {
        if matches!(self.state, State::Done { .. } | State::NoSpeech) {
            return self.state;
        }
        let level = rms(frame);
        let index = self.frames;
        self.frames += 1;

        if index < CALIBRATION_FRAMES {
            self.noise += level / CALIBRATION_FRAMES as f32;
            return self.state;
        }
        let threshold = (self.noise * NOISE_MARGIN).max(MIN_SPEECH_RMS);

        match self.speech_start {
            None => {
                if level > threshold {
                    self.loud_run += 1;
                    if self.loud_run >= START_FRAMES {
                        self.speech_start = Some(index + 1 - START_FRAMES);
                        self.state = State::Speaking;
                    }
                } else {
                    self.loud_run = 0;
                    // Follow slow changes in the background noise while waiting.
                    self.noise = self.noise * 0.95 + level * 0.05;
                }
                if self.speech_start.is_none() && index >= WAIT_FRAMES {
                    self.state = State::NoSpeech;
                }
            }
            Some(start) => {
                // A little hysteresis: trailing soft sounds still count as speech.
                self.quiet_run = if level < threshold * 0.7 {
                    self.quiet_run + 1
                } else {
                    0
                };
                let ended = self.quiet_run >= END_FRAMES;
                if ended || index + 1 - start >= MAX_SPEECH_FRAMES {
                    let last = if ended {
                        index + 1 - END_FRAMES
                    } else {
                        index + 1
                    };
                    self.state = State::Done {
                        start: start.saturating_sub(PAD_FRAMES) * self.frame_len,
                        end: (last + PAD_FRAMES).min(index + 1) * self.frame_len,
                    };
                }
            }
        }
        self.state
    }
}

/// Loudness of a frame as 0.0..1.0 on a rough logarithmic scale, for a level meter.
pub fn meter(frame: &[i16]) -> f32 {
    let db = 20.0 * (rms(frame).max(1.0) / 32768.0).log10();
    ((db + 60.0) / 50.0).clamp(0.0, 1.0)
}

fn rms(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum: f64 = frame.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / frame.len() as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16000;

    /// Low noise, then a loud tone (standing in for a voice), then noise again.
    fn signal(noise_s: f32, voice_s: f32, after_s: f32) -> Vec<i16> {
        let n = |s: f32| (s * RATE as f32) as usize;
        let mut out = Vec::new();
        let mut seed = 1u32;
        let mut noise = |amp: f32| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            ((seed >> 16) as f32 / 65535.0 - 0.5) * 2.0 * amp
        };
        for _ in 0..n(noise_s) {
            out.push(noise(150.0) as i16);
        }
        for i in 0..n(voice_s) {
            let t = i as f32 / RATE as f32;
            out.push(((t * 220.0 * std::f32::consts::TAU).sin() * 6000.0 + noise(150.0)) as i16);
        }
        for _ in 0..n(after_s) {
            out.push(noise(150.0) as i16);
        }
        out
    }

    fn run(samples: &[i16]) -> State {
        let mut d = Detector::new(RATE);
        let mut state = State::Waiting;
        for frame in samples.chunks_exact(d.frame_len()) {
            state = d.feed(frame);
        }
        state
    }

    #[test]
    fn finds_a_spoken_answer() {
        let State::Done { start, end } = run(&signal(1.0, 0.8, 1.5)) else {
            panic!("no answer found");
        };
        let (start, end) = (start as f32 / RATE as f32, end as f32 / RATE as f32);
        // Speech is 1.0..1.8 s; the answer may include up to 0.2 s of padding on each side.
        assert!((0.75..=1.0).contains(&start), "start {start}");
        assert!((1.8..=2.05).contains(&end), "end {end}");
    }

    #[test]
    fn gives_up_on_silence() {
        assert_eq!(run(&signal(9.0, 0.0, 0.0)), State::NoSpeech);
    }

    #[test]
    fn cuts_off_endless_speech() {
        let State::Done { start, end } = run(&signal(0.5, 8.0, 0.0)) else {
            panic!("no answer found");
        };
        let length = (end - start) as f32 / RATE as f32;
        assert!(length <= 6.3, "{length} s");
    }

    #[test]
    fn ignores_a_click() {
        // 40 ms of sound is shorter than START_FRAMES.
        assert_eq!(run(&signal(1.0, 0.04, 8.0)), State::NoSpeech);
    }

    #[test]
    fn meter_spans_silence_to_loud() {
        assert_eq!(meter(&[0; 320]), 0.0);
        assert!(meter(&[20000; 320]) > 0.9);
    }
}
