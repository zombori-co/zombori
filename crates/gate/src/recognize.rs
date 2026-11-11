//! Speech to text with Whisper (whisper.cpp), fully offline. The `base` model recognizes the
//! Ukrainian answer reliably in well under a second; `tiny` mishears it.

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// The sample rate Whisper works at.
pub const RATE: u32 = 16000;

pub struct Recognizer {
    context: WhisperContext,
}

impl Recognizer {
    /// Load a ggml model file (ggml-base.bin, about 150 MB).
    pub fn load(model: &str) -> Result<Self, String> {
        // Route whisper.cpp's chatter into the `log` facade, which nothing listens to.
        whisper_rs::install_logging_hooks();
        let context = WhisperContext::new_with_params(model, WhisperContextParameters::default())
            .map_err(|e| format!("loading {model}: {e}"))?;
        Ok(Self { context })
    }

    /// Transcribe mono samples at `RATE` (-1.0..1.0), as Ukrainian.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        // Whisper needs at least a second of audio; pad short answers with silence.
        let mut audio = samples.to_vec();
        audio.resize(audio.len().max(RATE as usize * 2), 0.0);

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("uk"));
        params.set_n_threads(
            std::thread::available_parallelism().map_or(2, |n| n.get().min(8)) as i32,
        );
        params.set_single_segment(true);
        params.set_no_context(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.context.create_state().map_err(|e| e.to_string())?;
        state.full(params, &audio).map_err(|e| e.to_string())?;
        let text: String = state.as_iter().map(|segment| segment.to_string()).collect();
        Ok(text.trim().to_string())
    }
}
