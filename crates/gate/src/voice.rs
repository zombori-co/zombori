//! The spoken side of a conversation, shared by the gate and the tavern: loading clips, recording
//! an answer, the "no" tone. Recording needs ALSA, so it is Linux only.

use audio::wav::Wav;

use crate::recognize::RATE;

/// Microphone audio ignored right after a question, while its echo dies down.
pub const SETTLE_S: f32 = 0.6;

/// Load a 16-bit WAV clip, or `None` (with the reason logged under `who`) if it is missing or
/// unreadable.
pub fn load(path: &str, who: &str) -> Option<Wav> {
    let bytes = std::fs::read(path).ok()?;
    match audio::wav::parse(&bytes) {
        Ok(wav) => Some(wav),
        Err(e) => {
            eprintln!("{who}: {path}: {e} (it must be 16-bit PCM WAV)");
            None
        }
    }
}

/// A short falling two-note "no", at `RATE`.
pub fn wrong_tone() -> Vec<i16> {
    let note = |freq: f32, seconds: f32| {
        let n = (seconds * RATE as f32) as usize;
        (0..n).map(move |i| {
            let t = i as f32 / RATE as f32;
            let envelope = (t / 0.01).min(1.0) * (1.0 - i as f32 / n as f32);
            ((t * freq * std::f32::consts::TAU).sin() * envelope * 9000.0) as i16
        })
    };
    note(392.0, 0.16).chain(note(277.0, 0.28)).collect()
}

/// 16-bit samples as floats in -1.0..1.0, the way the recognizer takes them.
pub fn to_f32(samples: &[i16]) -> Vec<f32> {
    samples.iter().map(|&s| f32::from(s) / 32768.0).collect()
}

/// Record until the speaker has finished: `None` if nobody spoke. `level` gets the microphone
/// level (0.0..1.0) for every 20 ms of audio, for a meter.
#[cfg(target_os = "linux")]
pub fn listen(mut level: impl FnMut(f32)) -> Result<Option<Vec<i16>>, String> {
    use audio::speech::{Detector, State, meter};

    let mic = audio::Microphone::open(RATE).map_err(|e| format!("microphone: {e}"))?;
    let mut detector = Detector::new(RATE);
    let mut frame = vec![0i16; detector.frame_len()];
    let mut recorded = Vec::new();
    // The question's tail can come back through the microphone (from the speakers, and late
    // through the VM's microphone stream); let it die down instead of taking it as an answer.
    let settle = (SETTLE_S * RATE as f32) as usize / frame.len();
    for _ in 0..settle {
        mic.read(&mut frame)
            .map_err(|e| format!("microphone: {e}"))?;
        level(meter(&frame));
    }
    loop {
        mic.read(&mut frame)
            .map_err(|e| format!("microphone: {e}"))?;
        recorded.extend_from_slice(&frame);
        level(meter(&frame));
        match detector.feed(&frame) {
            State::Done { start, end } => return Ok(Some(recorded[start..end].to_vec())),
            State::NoSpeech => return Ok(None),
            State::Waiting | State::Speaking => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_tone_is_short_and_audible() {
        let tone = wrong_tone();
        let seconds = tone.len() as f32 / RATE as f32;
        assert!((0.3..0.6).contains(&seconds), "{seconds} s");
        assert!(tone.iter().any(|&s| s.unsigned_abs() > 5000));
    }

    #[test]
    fn missing_clip_is_none() {
        assert!(load("/nonexistent/clip.wav", "test").is_none());
    }
}
