use nih_plug::prelude::*;
use std::sync::Arc;

pub struct Bitcrush {
    params: Arc<BitcrushParams>,
}

#[derive(Params)]
struct BitcrushParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "bits"]
    pub bits: FloatParam,

    #[id = "constant"]
    pub constant: FloatParam,
}

impl Default for Bitcrush {
    fn default() -> Self {
        Self {
            params: Arc::new(BitcrushParams::default()),
        }
    }
}

impl Default for BitcrushParams {
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

            bits: FloatParam::new(
                "Bits",
                16.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 16.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            constant: FloatParam::new(
                "Floating point constant",
                16.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1_000_000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
        }
    }
}

fn round_to_multiple(number: f32, multiple: f32) -> f32 {
    multiple * (number / multiple).round()
}

fn bitcrush_sample(input: f32, bits: f32) -> f32 {
    round_to_multiple(input, 2_f32.powf(-bits))
}

fn floating_point_quantize(input: f32, constant: f32) -> f32 {
    input + constant - constant
}

impl Plugin for Bitcrush {
    const NAME: &'static str = "Bitcrush v0.0.2";
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
            let gain = self.params.gain.smoothed.next();
            let bits = self.params.bits.smoothed.next();
            let constant = self.params.constant.smoothed.next();

            for sample in channel_samples {
                // Dynamic range quantization
                *sample = bitcrush_sample(*sample, bits);

                // Floating point error quantization
                *sample = floating_point_quantize(*sample, constant);

                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Bitcrush {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A simple bitcrusher");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for Bitcrush {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2_bitcrush";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nih_export_vst3!(Bitcrush);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitcrush_is_correct_4bits() {
        let inputs = vec![0., 0.1, 0.2, 0.5, 0.87, 1.0];
        let bits = 4.;
        let outputs: Vec<f32> = inputs.iter().map(|x| bitcrush_sample(*x, bits)).collect();
        let expected: Vec<f32> = vec![0.0, 0.125, 0.1875, 0.5, 0.875, 1.0];
        assert_eq!(expected, outputs);
    }

    #[test]
    fn bitcrush_is_correct_2bits() {
        let inputs = vec![0., 0.1, 0.2, 0.5, 0.87, 1.0];
        let bits = 2.;
        let outputs: Vec<f32> = inputs.iter().map(|x| bitcrush_sample(*x, bits)).collect();
        let expected: Vec<f32> = vec![0.0, 0.0, 0.25, 0.5, 0.75, 1.0];
        assert_eq!(expected, outputs);
    }

    #[test]
    fn bitcrush_is_correct_7bits() {
        let inputs = vec![0., 0.1, 0.2, 0.5, 0.87, 1.0];
        let bits = 7.;
        let outputs: Vec<f32> = inputs.iter().map(|x| bitcrush_sample(*x, bits)).collect();
        let expected: Vec<f32> = vec![0.0, 0.1015625, 0.203125, 0.5, 0.8671875, 1.0];
        assert_eq!(expected, outputs);
    }

    #[test]
    fn test_floating_point_quantize() {
        let inputs = vec![0., 0.1, 0.2, 0.5, 0.87, 1.0];
        let constant: f32 = 128.0;
        let outputs: Vec<f32> = inputs
            .iter()
            .map(|x| floating_point_quantize(*x, constant))
            .collect();
        assert_ne!(inputs, outputs);
        println!("{:?}", outputs);
    }

    #[test]
    fn test_floating_point_quantize_large_constant() {
        let inputs = vec![0., 0.1, 0.2, 0.5, 0.87, 1.0];
        let constant: f32 = 10000.0;
        let outputs: Vec<f32> = inputs
            .iter()
            .map(|x| floating_point_quantize(*x, constant))
            .collect();
        assert_ne!(inputs, outputs);
        println!("{:?}", outputs);
    }
}
