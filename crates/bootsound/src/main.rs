//! Zombori boot sound: plays the jingle once through ALSA, the kernel's sound API, then exits.
//! The jingle is compiled into the binary, so nothing has to be on disk at boot.

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod wav;

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
    use alsa::pcm::{Access, Format, HwParams, PCM};
    use alsa::{Direction, ValueOr};

    let jingle = wav::parse(JINGLE)?;

    // "default" lets ALSA convert rate and channels if the card can't take ours directly.
    let pcm = PCM::new("default", Direction::Playback, false)?;
    {
        let hw = HwParams::any(&pcm)?;
        hw.set_channels(jingle.channels.into())?;
        hw.set_rate(jingle.rate, ValueOr::Nearest)?;
        hw.set_format(Format::s16())?;
        hw.set_access(Access::RWInterleaved)?;
        pcm.hw_params(&hw)?;
    }
    let io = pcm.io_i16()?;

    // writei blocks while the card's buffer is full, and may take only part of what we offer.
    let channels = usize::from(jingle.channels);
    let mut rest = &jingle.samples[..];
    while !rest.is_empty() {
        match io.writei(rest) {
            Ok(frames) => rest = &rest[frames * channels..],
            // An underrun (we fell behind and the buffer ran dry): reset the stream and go on.
            Err(e) => pcm.try_recover(e, true)?,
        }
    }
    // Wait until the last sample has actually been played, not just handed over.
    pcm.drain()?;
    Ok(())
}
