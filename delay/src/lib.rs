use fx::delay_line::DelayLine;
use fx::DEFAULT_SAMPLE_RATE;
use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_DELAY_TIME_SECONDS: f32 = 5.0;

pub struct Delay {
    params: Arc<DelayParams>,
    delay_line_l: DelayLine,
    delay_line_r: DelayLine,
    should_update_delay_line: Arc<AtomicBool>,
}

#[derive(Params)]
struct DelayParams {
    #[id = "feedback"]
    pub feedback: FloatParam,

    #[id = "dry-wet-ratio"]
    pub dry_wet_ratio: FloatParam,

    #[id = "delay-time"]
    pub delay_time: FloatParam,
}

impl Default for Delay {
    fn default() -> Self {
        let should_update_delay_line = Arc::new(AtomicBool::new(true));
        Self {
            params: Arc::new(DelayParams::new(should_update_delay_line.clone())),
            should_update_delay_line,
            delay_line_l: DelayLine::new(
                DEFAULT_SAMPLE_RATE * MAX_DELAY_TIME_SECONDS as usize,
                DEFAULT_SAMPLE_RATE,
            ),
            delay_line_r: DelayLine::new(
                DEFAULT_SAMPLE_RATE * MAX_DELAY_TIME_SECONDS as usize,
                DEFAULT_SAMPLE_RATE,
            ),
        }
    }
}

impl DelayParams {
    fn new(should_update_delay_line: Arc<AtomicBool>) -> Self {
        Self {
            feedback: FloatParam::new("Feedback", 0.5, FloatRange::Linear { min: 0.0, max: 1.2 })
                .with_callback(Arc::new({
                    let should_update_delay_line = should_update_delay_line.clone();
                    move |_| should_update_delay_line.store(true, Ordering::SeqCst)
                }))
                .with_smoother(SmoothingStyle::Linear(1.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(2))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            dry_wet_ratio: FloatParam::new(
                "Dry/wet",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_callback(Arc::new({
                let should_update_delay_line = should_update_delay_line.clone();
                move |_| should_update_delay_line.store(true, Ordering::SeqCst)
            }))
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            delay_time: FloatParam::new(
                "Time",
                300.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: MAX_DELAY_TIME_SECONDS * 1000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_callback(Arc::new({
                let should_update_delay_line = should_update_delay_line.clone();
                move |_| should_update_delay_line.store(true, Ordering::SeqCst)
            }))
            .with_smoother(SmoothingStyle::Linear(2.0))
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
        }
    }
}

impl Plugin for Delay {
    const NAME: &'static str = "Delay v0.0.4";
    const VENDOR: &'static str = "Renzo Ledesma";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "renzol2@illinois.edu";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        let fs = _buffer_config.sample_rate;
        self.delay_line_l
            .resize_buffer((fs * MAX_DELAY_TIME_SECONDS) as usize);
        self.delay_line_l
            .set_delay_time(self.params.delay_time.value(), fs);
        self.delay_line_r
            .resize_buffer((fs * MAX_DELAY_TIME_SECONDS) as usize);
        self.delay_line_r
            .set_delay_time(self.params.delay_time.value(), fs);
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sample_rate = _context.transport().sample_rate;
        if self
            .should_update_delay_line
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            // Set delay time and feedback if params have changed
            let delay_time_ms = self.params.delay_time.smoothed.next();
            let feedback = self.params.feedback.smoothed.next();
            let dry_wet = self.params.dry_wet_ratio.smoothed.next();

            // Set both delay lines
            self.delay_line_l.set_delay_time(delay_time_ms, sample_rate);
            self.delay_line_l.set_feedback(feedback);
            self.delay_line_l.set_dry_wet(1.0 - dry_wet, dry_wet);
            self.delay_line_r.set_delay_time(delay_time_ms, sample_rate);
            self.delay_line_r.set_feedback(feedback);
            self.delay_line_r.set_dry_wet(1.0 - dry_wet, dry_wet);
        }
        for mut channel_samples in buffer.iter_samples() {
            // Set parameters while smoothing
            if self.params.delay_time.smoothed.is_smoothing() {
                let delay_time_ms = self.params.delay_time.smoothed.next();
                self.delay_line_l.set_delay_time(delay_time_ms, sample_rate);
                self.delay_line_r.set_delay_time(delay_time_ms, sample_rate);
            }
            if self.params.feedback.smoothed.is_smoothing() {
                let feedback = self.params.feedback.smoothed.next();
                self.delay_line_l.set_feedback(feedback);
                self.delay_line_r.set_feedback(feedback);
            }
            if self.params.dry_wet_ratio.smoothed.is_smoothing() {
                let dry_wet = self.params.dry_wet_ratio.smoothed.next();
                self.delay_line_l.set_dry_wet(1.0 - dry_wet, dry_wet);
                self.delay_line_r.set_dry_wet(1.0 - dry_wet, dry_wet);
            }

            // Process input
            let sample_l = *channel_samples.get_mut(0).unwrap();
            let sample_r = *channel_samples.get_mut(1).unwrap();

            let processed_l = self.delay_line_l.process_with_delay(sample_l);
            let processed_r = self.delay_line_r.process_with_delay(sample_r);

            *channel_samples.get_mut(0).unwrap() = processed_l;
            *channel_samples.get_mut(1).unwrap() = processed_r;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Delay {
    const CLAP_ID: &'static str = "com.your-domain.delay";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A basic feedback delay");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Delay {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2____delay";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(Delay);
