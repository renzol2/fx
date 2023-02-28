use nih_plug::prelude::*;
use std::sync::Arc;

struct BiquadFilter {
    // Coefficients
    a0: f32,
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,

    // Unit delays
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    fn new(frequency: f32) -> BiquadFilter {
        BiquadFilter::get_lowpass_filter_coefficients(frequency, 1.0)
    }

    fn get_lowpass_filter_coefficients(frequency: f32, q: f32) -> BiquadFilter {
        let alpha = (frequency.sin()) / (2.0 * q);
        let b0 = (1.0 - frequency.cos()) / 2.0;
        let b1 = 1.0 - frequency.cos();
        let b2 = (1.0 - frequency.cos()) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * frequency.cos();
        let a2 = 1.0 - alpha;
        BiquadFilter {
            a0,
            a1,
            a2,
            b0,
            b1,
            b2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn update_filter_coefficients(&mut self, frequency: f32, q: f32) {
        let alpha = (frequency.sin()) / (2.0 * q);
        self.b0 = (1.0 - frequency.cos()) / 2.0;
        self.b1 = 1.0 - frequency.cos();
        self.b2 = (1.0 - frequency.cos()) / 2.0;
        self.a0 = 1.0 + alpha;
        self.a1 = -2.0 * frequency.cos();
        self.a2 = 1.0 - alpha;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = input * self.a0 + self.z1;
        self.z1 = input * self.a1 + self.z2 - self.b1 * output;
        self.z2 = input * self.a2 - self.b2 * output;
        return output;
    }
}

pub struct Equalizer {
    params: Arc<EqualizerParams>,
    biquad: BiquadFilter,
    should_update_filter: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Params)]
struct EqualizerParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "cutoff-frequency"]
    pub cutoff_frequency: FloatParam,

    #[id = "q"]
    pub q: FloatParam,
}

impl Default for Equalizer {
    fn default() -> Self {
        Self {
            params: Arc::new(EqualizerParams::default()),
            biquad: BiquadFilter::new(
                EqualizerParams::default()
                    .cutoff_frequency
                    .default_normalized_value(),
            ),
            should_update_filter: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl Default for EqualizerParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            cutoff_frequency: FloatParam::new(
                "Cutoff",
                1_000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz"),

            q: FloatParam::new(
                "Q",
                1.0,
                FloatRange::Linear {
                    min: 0.1,
                    max: 30.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0)),
        }
    }
}

impl Plugin for Equalizer {
    const NAME: &'static str = "Equalizer v0.0.2";
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
        // are generated as needed. This layout will be called 'Stereo', while a layout with // only one input and output channel would be called 'Mono'.
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
        // Check if we should update filter coefficients
        if self
            .should_update_filter
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            let frequency = self.params.cutoff_frequency.smoothed.next();
            let q = self.params.q.smoothed.next();
            self.biquad.update_filter_coefficients(frequency, q);
        }

        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                let processed = self.biquad.process(*sample);
                *sample *= processed * gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Equalizer {
    const CLAP_ID: &'static str = "com.your-domain.equalizer";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A simple parametric EQ");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Equalizer,
    ];
}

impl Vst3Plugin for Equalizer {
    const VST3_CLASS_ID: [u8; 16] = *b"equalizerrenzol2";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Eq];
}

// nih_export_clap!(Equalizer);
nih_export_vst3!(Equalizer);
