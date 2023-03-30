use nih_plug::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod delay_line;
use delay_line::DelayLine;

const MAX_DELAY_TIME_SECONDS: f32 = 5.0;

pub struct Delay {
    params: Arc<DelayParams>,
    delay_line: DelayLine,
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
            delay_line: DelayLine::new(44100 * MAX_DELAY_TIME_SECONDS as usize),
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
    const NAME: &'static str = "Delay v0.0.1";
    const VENDOR: &'static str = "Renzo Ledesma";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "renzol2@illinois.edu";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
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
        self.delay_line
            .resize_buffer((fs * MAX_DELAY_TIME_SECONDS) as usize);
        self.delay_line
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
            self.delay_line.set_delay_time(delay_time_ms, sample_rate);
            self.delay_line.set_feedback(feedback);
            self.delay_line.set_dry_wet(1.0 - dry_wet, dry_wet);
        }
        for channel_samples in buffer.iter_samples() {
            // Set parameters while smoothing
            if self.params.delay_time.smoothed.is_smoothing() {
                let delay_time_ms = self.params.delay_time.smoothed.next();
                self.delay_line.set_delay_time(delay_time_ms, sample_rate)
            }
            if self.params.feedback.smoothed.is_smoothing() {
                let feedback = self.params.feedback.smoothed.next();
                self.delay_line.set_feedback(feedback);
            }
            if self.params.dry_wet_ratio.smoothed.is_smoothing() {
                let dry_wet = self.params.dry_wet_ratio.smoothed.next();
                self.delay_line.set_dry_wet(1.0 - dry_wet, dry_wet);
            }

            // Apply delay
            for sample in channel_samples {
                *sample = self.delay_line.process(*sample);
            }
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
