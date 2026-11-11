//! Zombori tavern: the session after the gate. The man (in front of the tavern's wooden wall for
//! now) asks which room you want; say a room's name from /etc/zombori/rooms.toml and he bows, the
//! picture fades, and the room's program opens full screen. Close it to come back to him.
//!
//! Programs run inside cage, a kiosk Wayland compositor that shows one program full screen. The
//! tavern draws straight to the GPU itself, so it hands the screen over while a program runs and
//! takes it back afterwards.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("tavern draws through Linux DRM/KMS and listens through ALSA; run it inside the VM");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = linux::run() {
        eprintln!("tavern: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::process::Command;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use gate::recognize::{RATE, Recognizer};
    use gate::scene::{Backdrop, OPENING_S, Scene, View, WRONG_S};
    use gate::voice::{self, load, to_f32, wrong_tone};
    use tavern::rooms::{self, Room};
    use theme::{VIRTUAL_H, VIRTUAL_W};

    const MODEL: &str = "/usr/local/share/zombori/ggml-base.bin";
    /// The man's question in the tavern ("which room?"): your own clip, like the gate's.
    const QUESTION: &str = "/usr/local/share/zombori/room-question.wav";
    const ROOMS: &str = "/etc/zombori/rooms.toml";

    #[derive(Clone, Copy, PartialEq)]
    enum Stage {
        /// `true` when he (re)appears and fades in.
        Asking(bool),
        Listening,
        Thinking,
        Wrong,
        /// Bowing toward the chosen room (an index into the rooms) before it opens.
        Opening(usize),
        /// The room's program is running; the tavern is off screen.
        Away,
    }

    /// What the conversation thread and the drawing loop share.
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
        let text = std::fs::read_to_string(ROOMS).map_err(|e| format!("{ROOMS}: {e}"))?;
        let rooms = rooms::parse(&text).map_err(|e| format!("{ROOMS}: {e}"))?;
        if rooms.is_empty() {
            return Err(format!("{ROOMS} has no rooms"));
        }

        let link: Link = Arc::new(Mutex::new(Shared {
            stage: Stage::Asking(true), // shown (fading in) while the model loads
            since: Instant::now(),
            level: 0.0,
            error: None,
        }));
        let worker = Arc::clone(&link);
        let worker_rooms = rooms.clone();
        thread::spawn(move || {
            if let Err(e) = converse(&worker, &worker_rooms) {
                worker.lock().unwrap().error = Some(e);
            }
        });

        let scene = Scene::with_backdrop(Backdrop::Tavern);
        let mut screen = open_screen()?;
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
                let t = shared.since.elapsed().as_secs_f32();
                (shared.stage, t, shared.level)
            };
            let view = match stage {
                Stage::Asking(first) => View::Asking { t, first },
                Stage::Listening => View::Listening { t, level },
                Stage::Thinking => View::Thinking { t },
                Stage::Wrong => View::Wrong { t },
                Stage::Opening(_) if t < OPENING_S => View::Opening { t },
                Stage::Opening(room) => {
                    // The picture has faded out: hand the screen to the room's program.
                    set(&link, Stage::Away);
                    drop(screen);
                    open_room(&rooms[room]);
                    screen = open_screen()?;
                    set(&link, Stage::Asking(true));
                    next_frame = Instant::now();
                    continue;
                }
                Stage::Away => View::Opening { t: OPENING_S },
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

    fn open_screen() -> Result<screen::Screen, String> {
        screen::Screen::open(Duration::from_secs(10)).map_err(|e| format!("screen: {e}"))
    }

    /// Run the room's program full screen in cage, until it is closed.
    fn open_room(room: &Room) {
        eprintln!("tavern: opening {} ({})", room.name, room.command.join(" "));
        let status = Command::new("cage")
            .arg("-s") // allow switching to another console (Ctrl+Alt+F2) while it runs
            .arg("--")
            .args(&room.command)
            // Virtual GPUs often draw the hardware cursor wrong; let the compositor draw it.
            .env("WLR_NO_HARDWARE_CURSORS", "1")
            .status();
        match status {
            Ok(status) if status.success() => eprintln!("tavern: back from {}", room.name),
            Ok(status) => eprintln!("tavern: {} ended with {status}", room.name),
            Err(e) => eprintln!("tavern: could not start cage ({e}); is it installed?"),
        }
    }

    /// Ask, listen and open rooms, forever.
    fn converse(link: &Link, rooms: &[Room]) -> Result<(), String> {
        let recognizer = Recognizer::load(MODEL)?;
        let question = load(QUESTION, "tavern");
        if question.is_none() {
            eprintln!("tavern: no question clip at {QUESTION}; listening without asking");
        }

        // The stage is already Asking (fading in) the first time and after coming back from a room.
        let mut stage_set = true;
        let mut ask = true;
        loop {
            if !stage_set {
                set(link, Stage::Asking(false));
            }
            stage_set = false;
            if ask {
                match &question {
                    Some(clip) => audio::play_wav(clip).map_err(|e| format!("playing: {e}"))?,
                    None => thread::sleep(Duration::from_millis(600)),
                }
            }

            set(link, Stage::Listening);
            let Some(answer) = voice::listen(|level| link.lock().unwrap().level = level)? else {
                // Nobody spoke: keep listening quietly instead of repeating the question.
                ask = false;
                continue;
            };

            set(link, Stage::Thinking);
            let heard = recognizer.transcribe(&to_f32(&answer))?;
            eprintln!("tavern: heard {heard:?}");
            let Some(index) =
                rooms::find(rooms, &heard).and_then(|room| rooms.iter().position(|r| r == room))
            else {
                set(link, Stage::Wrong);
                let started = Instant::now();
                let _ = audio::play(RATE, 1, &wrong_tone());
                thread::sleep(Duration::from_secs_f32(WRONG_S).saturating_sub(started.elapsed()));
                ask = true;
                continue;
            };

            // The drawing loop bows, opens the room, and sets Asking again when it is closed.
            set(link, Stage::Opening(index));
            while matches!(link.lock().unwrap().stage, Stage::Opening(_) | Stage::Away) {
                thread::sleep(Duration::from_millis(50));
            }
            stage_set = true;
            ask = true;
        }
    }
}
