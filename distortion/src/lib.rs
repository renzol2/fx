use nih_plug::prelude::*;
use std::sync::Arc;

pub struct Distortion {
    params: Arc<DistortionParams>,
}

#[derive(Enum, Debug, PartialEq, Eq)]
enum DistortionType {
    #[id = "soft-clipping"]
    #[name = "Soft clipping"]
    SoftClipping,

    #[id = "hard-clipping"]
    #[name = "Hard clipping"]
    HardClipping,

    #[id = "full-wave-rectifier"]
    #[name = "Full wave rectifier"]
    FullWaveRectifier,

    #[id = "half-wave-rectifier"]
    #[name = "Half wave rectifier"]
    HalfWaveRectifier,

    #[id = "shockley-diode-rectifier"]
    #[name = "Shockley diode rectifier"]
    ShockleyDiodeRectifier,

    #[id = "dropout"]
    #[name = "Dropout"]
    Dropout,

    #[id = "double-soft-clipper"]
    #[name = "Double soft clipper"]
    DoubleSoftClipper,
    
    #[id = "wavefolding"]
    #[name = "SineWavefolding"]
    Wavefolding,
}

#[derive(Params)]
struct DistortionParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "input-gain"]
    pub input_gain: FloatParam,

    #[id = "output-gain"]
    pub output_gain: FloatParam,

    #[id = "dry-wet"]
    pub dry_wet_ratio: FloatParam,

    #[id = "drive"]
    pub drive: FloatParam,

    #[id = "distortion-type"]
    pub distortion_type: EnumParam<DistortionType>,
}

impl Default for Distortion {
    fn default() -> Self {
        Self {
            params: Arc::new(DistortionParams::default()),
        }
    }
}

impl Default for DistortionParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            input_gain: FloatParam::new(
                "Input Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            output_gain: FloatParam::new(
                "Output Gain",
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

            drive: FloatParam::new(
                "Saturation",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.999,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            dry_wet_ratio: FloatParam::new(
                "Dry/wet",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            distortion_type: EnumParam::new("Type", DistortionType::SoftClipping),
        }
    }
}

impl Plugin for Distortion {
    const NAME: &'static str = "Distortion";
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
        for channel_samples in buffer.iter_samples() {
            let input_gain = self.params.input_gain.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();
            let a = self.params.drive.smoothed.next();
            let dry_wet_ratio = self.params.dry_wet_ratio.smoothed.next();
            let distortion_type = self.params.distortion_type.value();

            for sample in channel_samples {
                // Apply input gain
                *sample *= input_gain;

                // Apply distortion
                let wet = match distortion_type {
                    // https://www.musicdsp.org/en/latest/Effects/46-waveshaper.html
                    DistortionType::SoftClipping => {
                        let k = 2.0 * a / (1.0 - a);
                        ((1.0 + k) * *sample) / (1.0 + k * (*sample).abs())
                    },
                    DistortionType::HardClipping => {
                        let threshold = 1.;
                        if *sample > threshold {
                            threshold
                        } else if *sample < -threshold {
                            -threshold
                        } else {
                            *sample
                        }
                    },
                    DistortionType::FullWaveRectifier => {
                        (*sample).abs()
                    },
                    DistortionType::HalfWaveRectifier => {
                        if *sample < 0. { 0. } else { *sample }
                    },
                    DistortionType::ShockleyDiodeRectifier => {
                        // TODO: implement
                        *sample
                    },
                    DistortionType::Dropout => {
                        // TODO: implement
                        *sample
                    },
                    DistortionType::DoubleSoftClipper => {
                        // TODO: implement
                        *sample
                    },
                    DistortionType::Wavefolding => {
                        let k = 1. + (a * 4.);
                        (k * *sample).sin()
                    },
                };

                // Apply dry/wet
                *sample = (*sample * (1.0 - dry_wet_ratio)) + (wet * dry_wet_ratio);

                // Apply output gain
                *sample *= output_gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Distortion {
    const CLAP_ID: &'static str = "com.your-domain.distortion";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Algorithms of nonlinear systems for distortion effects");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for Distortion {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2_distortn";

    // And don't forget to change these categories, see the docstring on `VST3_CATEGORIES` for more
    // information
    const VST3_CATEGORIES: &'static str = "Fx|Distortion";
}

nih_export_clap!(Distortion);
nih_export_vst3!(Distortion);
