//! Zombori boot sound: plays the jingle once through ALSA, the kernel's sound API, then exits.
//! The jingle is compiled into the binary, so nothing has to be on disk at boot.

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const JINGLE: &[u8] = include_bytes!("../../../assets/sounds/boot.wav");

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("bootsound plays through Linux ALSA; build and run it inside the VM");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = run() {
        eprintln!("bootsound: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    audio::play_wav(&audio::wav::parse(JINGLE)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn jingle_parses() {
        let wav = audio::wav::parse(super::JINGLE).unwrap();
        // A few seconds long, and not silent.
        assert!((3.0..6.0).contains(&wav.seconds()), "{} s", wav.seconds());
        assert!(wav.samples.iter().any(|&s| s.unsigned_abs() > 1000));
    }
}
