use freeverb::Freeverb;
use nih_plug::prelude::*;
use std::sync::Arc;

mod freeverb;

pub struct Reverb {
    params: Arc<ReverbParams>,
    freeverb: Freeverb,
}

#[derive(Params)]
struct ReverbParams {
    #[id = "input-gain"]
    pub input_gain: FloatParam,

    #[id = "output-gain"]
    pub output_gain: FloatParam,

    #[id = "dry-wet"]
    pub dry_wet_ratio: FloatParam,

    #[id = "room-size"]
    pub room_size: FloatParam,

    #[id = "dampening"]
    pub damping: FloatParam,

    #[id = "frozen"]
    pub frozen: BoolParam,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            params: Arc::new(ReverbParams::default()),
            freeverb: Freeverb::new(44100), // default, can set later during initialization
        }
    }
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            input_gain: FloatParam::new(
                "Input gain",
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

            output_gain: FloatParam::new(
                "Output gain",
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

            dry_wet_ratio: FloatParam::new(
                "Dry/wet",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            room_size: FloatParam::new("Room size", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            damping: FloatParam::new("Dampening", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            frozen: BoolParam::new("Frozen", false),
        }
    }
}

impl Plugin for Reverb {
    const NAME: &'static str = "Reverb v0.0.3";
    const VENDOR: &'static str = "Renzo Ledesma";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "renzol2@illinois.edu";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const DEFAULT_INPUT_CHANNELS: u32 = 2;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;

    const DEFAULT_AUX_INPUTS: Option<AuxiliaryIOConfig> = None;
    const DEFAULT_AUX_OUTPUTS: Option<AuxiliaryIOConfig> = None;

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

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        self.freeverb
            .generate_filters(_buffer_config.sample_rate as usize);
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
        // Get parameters
        let room_size = self.params.room_size.smoothed.next();
        let dampening = self.params.damping.smoothed.next();

        // Update freeverb
        self.freeverb.set_room_size(room_size);
        self.freeverb.set_damping(dampening);

        // Process inputs
        for mut channel_samples in buffer.iter_samples() {
            // Update filter state with parameters
            if self.params.room_size.smoothed.is_smoothing() {
                self.freeverb
                    .set_room_size(self.params.room_size.smoothed.next());
            }
            if self.params.damping.smoothed.is_smoothing() {
                self.freeverb
                    .set_damping(self.params.damping.smoothed.next());
            }

            // Check if we should freeze the reverb
            let frozen = self.params.frozen.value();
            self.freeverb.set_frozen(frozen);

            // Get input/output gain
            let input_gain = self.params.input_gain.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();

            let in_l = *channel_samples.get_mut(0).unwrap();
            let in_r = *channel_samples.get_mut(1).unwrap();

            // Process with reverb
            let frame_out = self.freeverb.tick((in_l * input_gain, in_r * input_gain));

            // Apply dry/wet, then output
            let dry_wet_ratio = self.params.dry_wet_ratio.smoothed.next();
            let out_l = in_l * (1. - dry_wet_ratio) + frame_out.0 * dry_wet_ratio;
            let out_r = in_r * (1. - dry_wet_ratio) + frame_out.1 * dry_wet_ratio;

            *channel_samples.get_mut(0).unwrap() = out_l * output_gain;
            *channel_samples.get_mut(1).unwrap() = out_r * output_gain;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Reverb {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Algorithmic reverb effects");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Reverb,
    ];
}

impl Vst3Plugin for Reverb {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2___reverb";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Dynamics,
        Vst3SubCategory::Reverb,
    ];
}

// nih_export_clap!(Reverb);
nih_export_vst3!(Reverb);
