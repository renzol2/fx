use nih_plug::prelude::*;
use std::{
    f32::consts::{E, PI},
    sync::Arc,
};

pub struct Distortion {
    params: Arc<DistortionParams>,
}

#[derive(Enum, Debug, PartialEq, Eq)]
enum DistortionType {
    #[id = "saturation"]
    #[name = "Saturation"]
    Saturation,

    #[id = "hard-clipping"]
    #[name = "Hard clipping"]
    HardClipping,

    #[id = "fuzzy-rectifier"]
    #[name = "Fuzzy rectifier"]
    FuzzyRectifier,

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
    #[name = "Wavefolding"]
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
                "Drive",
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

            distortion_type: EnumParam::new("Type", DistortionType::Saturation),
        }
    }
}

impl Distortion {
    /// Processes an input sample through a static, saturating waveshaper.
    /// Drive parameter increases the saturation.
    ///
    /// Source: https://www.musicdsp.org/en/latest/Effects/46-waveshaper.html
    fn get_saturator_output(drive: f32, input_sample: f32) -> f32 {
        let drive = drive.min(0.99);
        let k = 2.0 * drive / (1.0 - drive);
        let wet = ((1.0 + k) * input_sample) / (1.0 + k * (input_sample).abs());
        // TODO: see how this sounds
        (1. - 0.3 * drive) * wet
    }

    /// Processes an input sample through a static hard clipper.
    /// Drive parameter increases distortion and reduces threshold.
    ///
    /// Desmos visualization of parameterization: https://www.desmos.com/calculator/7n1hzd53rf
    fn get_hard_clipper_output(drive: f32, input_sample: f32) -> f32 {
        let threshold = 1. - 0.5 * drive;
        let slope = 1. + 0.5 * drive;
        // Drive input into hard clipper for more distortion
        let x = input_sample * (1. + 4. * drive);
        if x.abs() < threshold {
            slope * x
        } else if slope * x > threshold {
            slope * threshold
        } else {
            -slope * threshold
        }
    }

    /// Processes an input sample through a fuzz inducing rectifier.
    /// Drive parameter linearly changes waveshaper from a half-wave rectifier to a full-wave rectifier.
    ///
    /// Desmos visualization of parameterization: https://www.desmos.com/calculator/ty0gtxg43u
    fn get_fuzzy_rectifier_output(drive: f32, input_sample: f32) -> f32 {
        let x = input_sample;
        if x >= 0. {
            input_sample
        } else {
            (1. - 2. * drive) * x
        }
    }

    /// FIXME: slightly increase range for more distortion
    /// TODO: update desmos graph
    ///
    /// Processes an input sample through a rectifying curve modeled after a Shockley-Diode circuit.
    /// Drive parameter changes the intensity of the curve.
    ///
    /// Based off Chowdhury's Shockley Diode rectifier approximation:
    /// https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf
    ///
    /// Desmos visualization of parameterization: https://www.desmos.com/calculator/r7gyt947xh
    fn get_shockley_diode_rectifier_output(drive: f32, input_sample: f32) -> f32 {
        (0.4 * drive + 0.1) * (E.powf((2. + 2. * drive) * input_sample) - 1.)
    }

    /// Processes an input sample through a dropout curve modeled after analog circuit response, where
    /// lower input levels snap to zero.
    /// Drive parameter changes the threshold of dropout.
    ///
    /// Based off Chowdhury's Dropout equation:
    /// https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf
    ///
    /// Desmos visualization of parameterization: https://www.desmos.com/calculator/2dmj6p7yvk
    fn get_dropout_output(drive: f32, input_sample: f32) -> f32 {
        if drive == 0. {
            input_sample
        } else {
            let b = f32::sqrt(drive.powi(3) / 3.);
            let x = input_sample;
            if x < -b {
                x + b - (b / drive).powi(3)
            } else if -b <= x && x <= b {
                (x / drive).powi(3)
            } else {
                x - b + (b / drive).powi(3)
            }
        }
    }

    fn cubic_waveshaper(x: f32) -> f32 {
        (0.75) * (x - x.powi(3) / 3.)
    }

    fn lower_waveshaper(x: f32, lower_skew_param: f32) -> f32 {
        let b = lower_skew_param;
        let b_recip = 1. / b;
        if x < -b_recip {
            -(0.5)
        } else if x > b_recip {
            0.5
        } else {
            Self::cubic_waveshaper(lower_skew_param * x)
        }
    }

    /// Processes an input sample through an asymmetrical, "double soft clipper" waveshaper algorithm.
    /// The drive parameter changes the upper limit of positive inputs and the skew of negative inputs.
    ///
    /// Based off Chowdhury's double soft clipper:
    /// https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf
    /// Desmos visualization of parameterization: https://www.desmos.com/calculator/bplxqizjbe
    fn get_double_soft_clipper_output(drive: f32, input_sample: f32) -> f32 {
        let x = input_sample;
        let upper_limit_param = 1. - 0.4 * drive;
        let lower_skew_param = 2. * drive + 1.;
        if -1. <= x && x <= 0. {
            Self::lower_waveshaper(2. * x + 1., lower_skew_param) - 0.5
        } else if 0. < x && x <= 1. {
            // Drive input value
            let x = x * 1.5;
            upper_limit_param * (Self::cubic_waveshaper(2. * x - 1.) + 0.5)
        } else if x < -1. {
            -1.
        } else {
            1.
        }
    }

    /// Processes an input sample through a sinusoidal wavefolder.
    /// The drive parameter increases the frequency of the sine curve, causing more distortion.
    fn get_waveshaper_output(drive: f32, input_sample: f32) -> f32 {
        let k = 1. + (drive * 3.);
        let wet = (2. * PI * k * input_sample).sin();
        
        // Apply dry/wet based on drive to control volume
        let wet = (1. - drive) * input_sample + (drive) * wet;
        
        // TODO: check how this sounds now
        // Reduce gain as drive increases
        (1. - 0.3 * drive) * wet
    }
}

impl Plugin for Distortion {
    const NAME: &'static str = "Distortion v0.0.7";
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
            let drive = self.params.drive.smoothed.next();
            let dry_wet_ratio = self.params.dry_wet_ratio.smoothed.next();
            let distortion_type = self.params.distortion_type.value();

            // TODO: implement upsampling

            for sample in channel_samples {
                // Apply input gain
                *sample *= input_gain;

                // Apply distortion
                let wet = match distortion_type {
                    DistortionType::Saturation => Self::get_saturator_output(drive, *sample),
                    DistortionType::HardClipping => Self::get_hard_clipper_output(drive, *sample),
                    DistortionType::FuzzyRectifier => {
                        Self::get_fuzzy_rectifier_output(drive, *sample)
                    }
                    DistortionType::ShockleyDiodeRectifier => {
                        Self::get_shockley_diode_rectifier_output(drive, *sample)
                    }
                    DistortionType::Dropout => Distortion::get_dropout_output(drive, *sample),
                    DistortionType::DoubleSoftClipper => {
                        Self::get_double_soft_clipper_output(drive, *sample)
                    }
                    DistortionType::Wavefolding => Self::get_waveshaper_output(drive, *sample),
                };

                // Apply dry/wet
                *sample = (*sample * (1.0 - dry_wet_ratio)) + (wet * dry_wet_ratio);

                // Apply output gain
                *sample *= output_gain;
            }

            // TODO: implement downsampling
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Distortion {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
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

// TODO: write tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturator_returns_correct_dc_offset() {
        assert_eq!(Distortion::get_saturator_output(0., 0.), 0.);
    }
}

nih_export_clap!(Distortion);
nih_export_vst3!(Distortion);
