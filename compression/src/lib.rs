use fx::{dynamics::DynamicRangeProcessor, DEFAULT_SAMPLE_RATE};
use nih_plug::prelude::*;
use std::sync::Arc;

pub struct Compression {
    params: Arc<CompressionParams>,
    processor: DynamicRangeProcessor,
}

#[derive(Params)]
struct CompressionParams {
    #[id = "input-gain"]
    pub input_gain: FloatParam,
    #[id = "threshold"]
    pub threshold: FloatParam,
    #[id = "ratio"]
    pub ratio: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
    #[id = "makeup-gain"]
    pub makeup_gain: FloatParam,
    #[id = "dry-wet"]
    pub dry_wet: FloatParam,
    #[id = "use-expander"]
    pub use_expander: BoolParam,
}

impl Default for Compression {
    fn default() -> Self {
        Self {
            params: Arc::new(CompressionParams::default()),
            processor: DynamicRangeProcessor::new(DEFAULT_SAMPLE_RATE),
        }
    }
}

impl Default for CompressionParams {
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

            threshold: FloatParam::new(
                "Threshold",
                0.0,
                FloatRange::Linear {
                    min: -60.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_smoother(SmoothingStyle::Exponential(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            ratio: FloatParam::new(
                "Ratio",
                4.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-1.3),
                },
            )
            .with_unit(":1")
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            attack: FloatParam::new(
                "Attack",
                2.0,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            release: FloatParam::new(
                "Release",
                30.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            dry_wet: FloatParam::new("Dry/wet", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Exponential(50.0))
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            makeup_gain: FloatParam::new(
                "Makeup gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            use_expander: BoolParam::new("Compress/Expand", false),
        }
    }
}

impl Plugin for Compression {
    const NAME: &'static str = "Compression v0.0.3";
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
        let sample_rate = _buffer_config.sample_rate;
        self.processor.set_sample_rate(sample_rate as usize);
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
        for mut channel_samples in buffer.iter_samples() {
            // Update processor's parameters
            let threshold = self.params.threshold.smoothed.next();
            let ratio = self.params.ratio.smoothed.next();
            let attack = self.params.attack.smoothed.next() * 0.001; // convert from ms to s
            let release = self.params.release.smoothed.next() * 0.001; // convert from ms to s
            let is_expander = self.params.use_expander.value();
            self.processor
                .set_parameters(threshold, ratio, attack, release, is_expander);

            let input_gain = self.params.input_gain.smoothed.next();
            let in_l = *channel_samples.get_mut(0).unwrap() * input_gain;
            let in_r = *channel_samples.get_mut(1).unwrap() * input_gain;

            // Process
            let input = (in_l * input_gain, in_r * input_gain);
            let makeup_gain = self.params.makeup_gain.smoothed.next();
            let makeup_gain_db = util::gain_to_db_fast(makeup_gain);
            let frame_out = self.processor.process_input_frame(input, makeup_gain_db);

            // Apply dry/wet, then output
            let dry_wet_ratio = self.params.dry_wet.smoothed.next();
            let out_l = in_l * (1. - dry_wet_ratio) + frame_out.0 * dry_wet_ratio;
            let out_r = in_r * (1. - dry_wet_ratio) + frame_out.1 * dry_wet_ratio;

            *channel_samples.get_mut(0).unwrap() = out_l;
            *channel_samples.get_mut(1).unwrap() = out_r;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Compression {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A simple dynamic range compressor");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Compressor,
    ];
}

impl Vst3Plugin for Compression {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2_compress";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(Compression);
