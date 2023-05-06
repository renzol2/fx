use nih_plug::prelude::*;
use std::sync::Arc;

mod delay_line;
use delay_line::StereoChorus;

const MAX_DELAY_TIME_SECONDS: f32 = 5.0;
const DEFAULT_SAMPLE_RATE: usize = 44100;
const PARAMETER_MINIMUM: f32 = 0.01;

pub struct Chorus {
    params: Arc<ChorusParams>,
    chorus: StereoChorus,
}

#[derive(Params)]
struct ChorusParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "rate"]
    pub rate: FloatParam,

    #[id = "lfo-amount"]
    pub lfo_amount: FloatParam,

    #[id = "depth"]
    pub depth: FloatParam,

    #[id = "width"]
    pub width: FloatParam,

    #[id = "feedback"]
    pub feedback: FloatParam,
}

impl Default for Chorus {
    fn default() -> Self {
        Self {
            params: Arc::new(ChorusParams::default()),
            chorus: StereoChorus::new(MAX_DELAY_TIME_SECONDS, DEFAULT_SAMPLE_RATE),
        }
    }
}

impl Default for ChorusParams {
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

            rate: FloatParam::new(
                "Rate",
                0.1,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 3.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            lfo_amount: FloatParam::new(
                "LFO Amount",
                0.02,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 3.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" freq. ratio")
            .with_value_to_string(formatters::v2s_f32_rounded(3)),

            depth: FloatParam::new(
                "Depth",
                0.5,
                FloatRange::Linear {
                    min: PARAMETER_MINIMUM,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            width: FloatParam::new(
                "Width",
                0.5,
                FloatRange::Linear {
                    min: PARAMETER_MINIMUM,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            feedback: FloatParam::new(
                "Feedback",
                0.5,
                FloatRange::Linear {
                    min: PARAMETER_MINIMUM,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
        }
    }
}

impl Plugin for Chorus {
    const NAME: &'static str = "Chorus v0.0.3";
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
        self.chorus
            .resize_buffers(MAX_DELAY_TIME_SECONDS, _buffer_config.sample_rate as usize);
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for mut channel_samples in buffer.iter_samples() {
            // Get parameters
            let gain = self.params.gain.smoothed.next();
            let rate = self.params.rate.smoothed.next();
            let vibrato_width = self.params.lfo_amount.smoothed.next();
            let depth = self.params.depth.smoothed.next();
            let width = self.params.width.smoothed.next() * 0.5;
            let feedback = self.params.feedback.smoothed.next();

            // Process input
            let sample_l = *channel_samples.get_mut(0).unwrap();
            let sample_r = *channel_samples.get_mut(1).unwrap();

            let (processed_l, processed_r) = self.chorus.process_with_chorus(
                (sample_l, sample_r),
                rate,
                vibrato_width,
                width,
                depth,
                feedback,
            );

            *channel_samples.get_mut(0).unwrap() = processed_l * gain;
            *channel_samples.get_mut(1).unwrap() = processed_r * gain;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Chorus {
    const CLAP_ID: &'static str = "com.your-domain.chorus";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A traditional chorus effect");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Chorus {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2___chorus";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

// nih_export_clap!(Chorus);
nih_export_vst3!(Chorus);
