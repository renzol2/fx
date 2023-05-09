use fx::{
    delay_line::StereoDelay, DEFAULT_SAMPLE_RATE, FLUTTER_MAX_FREQUENCY_RATIO,
    FLUTTER_MAX_LFO_FREQUENCY, MAX_DELAY_TIME_SECONDS, WOW_MAX_FREQUENCY_RATIO,
    WOW_MAX_LFO_FREQUENCY,
};
use nih_plug::prelude::*;
use std::sync::Arc;

const PARAMETER_MINIMUM: f32 = 0.01;

pub struct Vibrato {
    params: Arc<VibratoParams>,
    wow_vibrato: StereoDelay,
    flutter_vibrato: StereoDelay,
}

#[derive(Params)]
struct VibratoParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "wow"]
    pub wow: FloatParam,

    #[id = "flutter"]
    pub flutter: FloatParam,

    #[id = "width"]
    pub width: FloatParam,
}

impl Default for Vibrato {
    fn default() -> Self {
        Self {
            params: Arc::new(VibratoParams::default()),
            wow_vibrato: StereoDelay::new(MAX_DELAY_TIME_SECONDS, DEFAULT_SAMPLE_RATE),
            flutter_vibrato: StereoDelay::new(MAX_DELAY_TIME_SECONDS, DEFAULT_SAMPLE_RATE),
        }
    }
}

impl Default for VibratoParams {
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

            wow: FloatParam::new(
                "Wow",
                0.3,
                FloatRange::Skewed {
                    min: PARAMETER_MINIMUM,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            flutter: FloatParam::new(
                "Flutter",
                0.3,
                FloatRange::Skewed {
                    min: PARAMETER_MINIMUM,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            width: FloatParam::new(
                "Width",
                0.0,
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

impl Plugin for Vibrato {
    const NAME: &'static str = "Vibrato v0.0.15";
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
        self.wow_vibrato
            .resize_buffers(MAX_DELAY_TIME_SECONDS, fs as usize);
        self.flutter_vibrato
            .resize_buffers(MAX_DELAY_TIME_SECONDS, fs as usize);
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
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();
            let wow = self.params.wow.smoothed.next();
            let flutter = self.params.flutter.smoothed.next();
            let width = self.params.width.smoothed.next();

            let phase_offset = width * 0.5; // only offset right phase by a maximum of 180 degrees
            let sample_l = *channel_samples.get_mut(0).unwrap();
            let sample_r = *channel_samples.get_mut(1).unwrap();

            let mut processed_samples = (sample_l, sample_r);

            // Apply wow
            if wow > PARAMETER_MINIMUM {
                processed_samples = self.wow_vibrato.process_with_vibrato(
                    processed_samples,
                    WOW_MAX_LFO_FREQUENCY,
                    wow * WOW_MAX_FREQUENCY_RATIO,
                    phase_offset,
                );
            }

            // Apply flutter
            if flutter > PARAMETER_MINIMUM {
                processed_samples = self.flutter_vibrato.process_with_vibrato(
                    processed_samples,
                    FLUTTER_MAX_LFO_FREQUENCY,
                    flutter * FLUTTER_MAX_FREQUENCY_RATIO,
                    phase_offset,
                );
            }

            *channel_samples.get_mut(0).unwrap() = processed_samples.0 * gain;
            *channel_samples.get_mut(1).unwrap() = processed_samples.1 * gain;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Vibrato {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A vibrato effect w/ wow & flutter");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Vibrato {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2__vibrato";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::PitchShift,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_vst3!(Vibrato);
