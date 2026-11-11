//! Just enough of the WAV format to read our own jingle: 16-bit PCM, any rate, any channel count.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jingle_parses() {
        let wav = parse(crate::JINGLE).unwrap();
        // A few seconds long, and not silent.
        let frames = wav.samples.len() / usize::from(wav.channels);
        let seconds = frames as f32 / wav.rate as f32;
        assert!((3.0..6.0).contains(&seconds), "{seconds} s");
        assert!(wav.samples.iter().any(|&s| s.unsigned_abs() > 1000));
    }

    #[test]
    fn rejects_other_files() {
        assert!(parse(b"not audio at all").is_err());
    }
}
