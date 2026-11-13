//! Playing and recording through ALSA's "default" device, which converts rate and channel count
//! when the card can't take ours directly.

use std::thread::sleep;
use std::time::{Duration, Instant};

use alsa::pcm::{Access, Format, HwParams, IO, PCM};
use alsa::{Direction, ValueOr};

use crate::wav::Wav;

type Result<T> = std::result::Result<T, alsa::Error>;

/// How long to keep trying a device that isn't ready yet. Early in boot a card can exist before
/// it can play: on Apple Silicon the speakers only open once speakersafetyd has unlocked them.
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// Half a second of buffer, as aplay and arecord use. Left alone, ALSA picks the largest buffer
/// the device allows, which a chain of plugins (format, mixing, rate) may fail to allocate.
const BUFFER_US: u32 = 500_000;

fn open(direction: Direction, rate: u32, channels: u16) -> Result<PCM> {
    let deadline = Instant::now() + OPEN_TIMEOUT;
    loop {
        match try_open(direction, rate, channels) {
            Err(_) if Instant::now() < deadline => sleep(Duration::from_millis(200)),
            result => return result,
        }
    }
}

fn try_open(direction: Direction, rate: u32, channels: u16) -> Result<PCM> {
    let pcm = PCM::new("default", direction, false)?;
    {
        let hw = HwParams::any(&pcm)?;
        hw.set_channels(channels.into())?;
        hw.set_rate(rate, ValueOr::Nearest)?;
        hw.set_format(Format::s16())?;
        hw.set_access(Access::RWInterleaved)?;
        hw.set_buffer_time_near(BUFFER_US, ValueOr::Nearest)?;
        pcm.hw_params(&hw)?;
    }
    Ok(pcm)
}

/// Play interleaved 16-bit samples and return once the last one has been heard.
pub fn play(rate: u32, channels: u16, samples: &[i16]) -> Result<()> {
    let pcm = open(Direction::Playback, rate, channels)?;
    let io = pcm.io_i16()?;
    // writei blocks while the card's buffer is full, and may take only part of what we offer.
    let channels = usize::from(channels);
    let mut rest = samples;
    while !rest.is_empty() {
        match io.writei(rest) {
            Ok(frames) => rest = &rest[frames * channels..],
            // An underrun (we fell behind and the buffer ran dry): reset the stream and go on.
            Err(e) => pcm.try_recover(e, true)?,
        }
    }
    // Wait until the last sample has actually been played, not just handed over.
    pcm.drain()
}

pub fn play_wav(wav: &Wav) -> Result<()> {
    play(wav.rate, wav.channels, &wav.samples)
}

/// The microphone, recording mono 16-bit samples.
pub struct Microphone {
    pcm: PCM,
}

impl Microphone {
    pub fn open(rate: u32) -> Result<Self> {
        let pcm = open(Direction::Capture, rate, 1)?;
        pcm.start()?;
        Ok(Self { pcm })
    }

    /// Fill `buf` with the next samples, blocking until they have been recorded.
    pub fn read(&self, buf: &mut [i16]) -> Result<()> {
        let io: IO<i16> = self.pcm.io_i16()?;
        let mut filled = 0;
        while filled < buf.len() {
            match io.readi(&mut buf[filled..]) {
                Ok(frames) => filled += frames,
                // An overrun (we didn't read fast enough): reset the stream and go on.
                Err(e) => self.pcm.try_recover(e, true)?,
            }
        }
        Ok(())
    }
}
