//! Zombori gate: before the login prompt, asks «Скажи мені, що таке Зомборі?» and listens. Only
//! the right answer («музика») lets the boot continue, with a head swaying to the music. A wrong
//! answer asks again.
//!
//! If anything needed is missing (screen, microphone, model), the gate logs why and exits with an
//! error, and the boot continues: a broken gate must never lock you out.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "gate draws through Linux DRM/KMS and listens through ALSA; build and run it inside the VM"
    );
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = linux::run() {
        eprintln!("gate: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use audio::wav::Wav;
    use gate::answer::is_correct;
    use gate::beat::{self, Beat};
    use gate::recognize::{RATE, Recognizer};
    use gate::scene::{Scene, View, WRONG_S};
    use gate::voice::{self, load, to_f32, wrong_tone};
    use theme::{VIRTUAL_H, VIRTUAL_W};

    /// Installed by system/dev-install.sh. The question and the music are your own clips.
    const MODEL: &str = "/usr/local/share/zombori/ggml-base.bin";
    const QUESTION: &str = "/usr/local/share/zombori/question.wav";
    const MUSIC: &str = "/usr/local/share/zombori/celebrate.wav";
    /// The celebration plays at most this much of the clip, fading out at the end.
    const MAX_CELEBRATION_S: f32 = 25.0;
    const FADE_OUT_S: f32 = 2.0;

    #[derive(Clone, Copy)]
    enum Stage {
        /// `true` the first time: the man fades in.
        Asking(bool),
        Listening,
        Thinking,
        Wrong,
        /// The beat, and when the music starts within the clip.
        Celebrating(Beat, f32),
        Done,
    }

    /// What the conversation thread tells the drawing loop.
    struct Shared {
        stage: Stage,
        since: Instant,
        level: f32,
        error: Option<String>,
    }

    type Link = Arc<Mutex<Shared>>;

    fn set(link: &Link, stage: Stage) {
        let mut shared = link.lock().unwrap();
        shared.stage = stage;
        shared.since = Instant::now();
    }

    pub fn run() -> Result<(), String> {
        let mut screen =
            screen::Screen::open(Duration::from_secs(10)).map_err(|e| format!("screen: {e}"))?;
        let link: Link = Arc::new(Mutex::new(Shared {
            stage: Stage::Asking(true), // shown (fading in) while the model loads
            since: Instant::now(),
            level: 0.0,
            error: None,
        }));
        let worker = Arc::clone(&link);
        thread::spawn(move || {
            if let Err(e) = converse(&worker) {
                worker.lock().unwrap().error = Some(e);
            }
        });

        let scene = Scene::new();
        let mut frame = vec![[0u8; 3]; (VIRTUAL_W * VIRTUAL_H) as usize];
        // Page flips normally pace us to the monitor, but QEMU's virtio-gpu completes them
        // immediately. Never draw faster than this.
        let frame_interval = Duration::from_secs(1) / 30;
        let mut next_frame = Instant::now();
        loop {
            let (stage, t, level) = {
                let shared = link.lock().unwrap();
                if let Some(e) = &shared.error {
                    return Err(e.clone());
                }
                (
                    shared.stage,
                    shared.since.elapsed().as_secs_f32(),
                    shared.level,
                )
            };
            let view = match stage {
                Stage::Asking(first) => View::Asking { t, first },
                Stage::Listening => View::Listening { t, level },
                Stage::Thinking => View::Thinking { t },
                Stage::Wrong => View::Wrong { t },
                Stage::Celebrating(beat, music) => View::Celebrating { t, beat, music },
                Stage::Done => return Ok(()),
            };
            scene.render(view, &mut frame);
            screen.show(&frame).map_err(|e| format!("screen: {e}"))?;

            next_frame += frame_interval;
            let now = Instant::now();
            match next_frame.checked_duration_since(now) {
                Some(wait) => thread::sleep(wait),
                None => next_frame = now, // running behind: don't try to catch up
            }
        }
    }

    /// Ask until the answer is right, then celebrate.
    fn converse(link: &Link) -> Result<(), String> {
        let recognizer = Recognizer::load(MODEL)?;
        let question = load(QUESTION, "gate");
        if question.is_none() {
            eprintln!("gate: no question clip at {QUESTION}; listening without asking");
        }

        let mut first = true;
        loop {
            // The first time, keep the stage (and its fade-in) that started with the program.
            if !first {
                set(link, Stage::Asking(false));
            }
            first = false;
            match &question {
                Some(clip) => audio::play_wav(clip).map_err(|e| format!("playing: {e}"))?,
                None => thread::sleep(Duration::from_millis(600)),
            }

            set(link, Stage::Listening);
            let Some(answer) = voice::listen(|level| link.lock().unwrap().level = level)? else {
                eprintln!("gate: heard nothing, asking again");
                continue;
            };

            set(link, Stage::Thinking);
            let heard = recognizer.transcribe(&to_f32(&answer))?;
            eprintln!("gate: heard {heard:?}");
            if is_correct(&heard) {
                break;
            }

            set(link, Stage::Wrong);
            let started = Instant::now();
            let _ = audio::play(RATE, 1, &wrong_tone()); // the shake still shows without sound
            thread::sleep(Duration::from_secs_f32(WRONG_S).saturating_sub(started.elapsed()));
        }

        celebrate(link)?;
        set(link, Stage::Done);
        Ok(())
    }

    /// Play the clip (or wait, without one) while the head sways to its beat.
    fn celebrate(link: &Link) -> Result<(), String> {
        let Some(clip) = load(MUSIC, "gate") else {
            eprintln!("gate: no music at {MUSIC}; celebrating in silence");
            set(link, Stage::Celebrating(beat::FALLBACK, 0.0));
            thread::sleep(Duration::from_secs(8));
            return Ok(());
        };
        // Analyze the whole clip, not just the part that plays: more music, a steadier tempo.
        let mono = clip.mono_f32();
        let beat = beat::detect(&mono, clip.rate);
        let music = beat::music_start(&mono, clip.rate);
        eprintln!(
            "gate: music from {music:.1} s at {:.1} BPM, first beat at {:.2} s",
            beat.bpm, beat.first
        );
        let clip = trim_and_fade(clip);
        set(link, Stage::Celebrating(beat, music));
        audio::play_wav(&clip).map_err(|e| format!("playing: {e}"))
    }

    /// Keep the first MAX_CELEBRATION_S of the music, fading out over its last FADE_OUT_S.
    fn trim_and_fade(mut wav: Wav) -> Wav {
        let channels = usize::from(wav.channels.max(1));
        let keep = (MAX_CELEBRATION_S * wav.rate as f32) as usize * channels;
        if wav.samples.len() > keep {
            wav.samples.truncate(keep);
            let fade = (FADE_OUT_S * wav.rate as f32) as usize * channels;
            let from = keep - fade.min(keep);
            for (i, s) in wav.samples[from..].iter_mut().enumerate() {
                let k = 1.0 - i as f32 / fade as f32;
                *s = (f32::from(*s) * k) as i16;
            }
        }
        wav
    }
}
