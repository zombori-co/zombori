//! Run the gate's recognizer and answer check on WAV files, to test them without a VM:
//!   cargo run -p gate --release --example transcribe -- ggml-base.bin answer1.wav answer2.wav
//! The WAV files must be 16-bit, 16 kHz (afconvert -f WAVE -d LEI16@16000 -c 1 in out.wav).

use gate::answer::is_correct;
use gate::recognize::{RATE, Recognizer};

fn main() {
    let mut args = std::env::args().skip(1);
    let model = args.next().expect("usage: transcribe <model> <wav>...");
    let recognizer = Recognizer::load(&model).unwrap();
    for path in args {
        let wav = audio::wav::parse(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(wav.rate, RATE, "{path}: needs {RATE} Hz");
        let start = std::time::Instant::now();
        let heard = recognizer.transcribe(&wav.mono_f32()).unwrap();
        let verdict = if is_correct(&heard) { "OPEN" } else { "no" };
        println!(
            "{verdict:>4}  {heard:?}  ({} ms)  {path}",
            start.elapsed().as_millis()
        );
    }
}
