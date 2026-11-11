//! Just enough of the WAV format to read our own sounds: 16-bit PCM, any rate, any channel count.
//! No hardware involved, so this runs (and is tested) anywhere.
//!
//! A WAV file is a RIFF container: a 12-byte header, then "chunks", each a 4-byte tag, a 4-byte
//! little-endian length and the data. We need two of them: "fmt " (the format) and "data".

pub struct Wav {
    pub rate: u32,
    pub channels: u16,
    /// Interleaved samples: for stereo, left, right, left, right...
    pub samples: Vec<i16>,
}

pub fn parse(bytes: &[u8]) -> Result<Wav, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let mut format = None;
    let mut rest = &bytes[12..];
    while rest.len() >= 8 {
        let tag = &rest[0..4];
        let len = u32::from_le_bytes(rest[4..8].try_into().unwrap()) as usize;
        let body = rest.get(8..8 + len).ok_or("truncated chunk")?;
        match tag {
            b"fmt " => {
                let field = |at: usize| u16::from_le_bytes([body[at], body[at + 1]]);
                if body.len() < 16 || field(0) != 1 || field(14) != 16 {
                    return Err("only 16-bit PCM is supported".into());
                }
                let rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                format = Some((rate, field(2)));
            }
            b"data" => {
                let (rate, channels) = format.ok_or("data chunk before fmt chunk")?;
                let samples = body
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&b| i16::from_le_bytes(b))
                    .collect();
                return Ok(Wav {
                    rate,
                    channels,
                    samples,
                });
            }
            _ => {}
        }
        // Chunks are padded to an even length.
        rest = &rest[(8 + len + len % 2).min(rest.len())..];
    }
    Err("no data chunk".into())
}

impl Wav {
    /// Length in seconds.
    pub fn seconds(&self) -> f32 {
        let frames = self.samples.len() / usize::from(self.channels.max(1));
        frames as f32 / self.rate as f32
    }

    /// The samples mixed down to one channel, as floats in -1.0..1.0.
    pub fn mono_f32(&self) -> Vec<f32> {
        let channels = usize::from(self.channels.max(1));
        self.samples
            .chunks_exact(channels)
            .map(|frame| {
                frame.iter().map(|&s| f32::from(s)).sum::<f32>() / channels as f32 / 32768.0
            })
            .collect()
    }
}

/// Build a 16-bit PCM WAV file in memory (for tests and generated sounds).
pub fn encode(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_stereo() {
        let samples = [0, 1000, -1000, i16::MAX, i16::MIN, 7];
        let wav = parse(&encode(44100, 2, &samples)).unwrap();
        assert_eq!((wav.rate, wav.channels), (44100, 2));
        assert_eq!(wav.samples, samples);
        assert!((wav.seconds() - 3.0 / 44100.0).abs() < 1e-9);
    }

    #[test]
    fn mixes_down_to_mono() {
        let wav = parse(&encode(16000, 2, &[16384, 0, -16384, -16384])).unwrap();
        assert_eq!(wav.mono_f32(), vec![0.25, -0.5]);
    }

    #[test]
    fn rejects_other_files() {
        assert!(parse(b"not audio at all").is_err());
    }
}
