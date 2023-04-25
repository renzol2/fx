use nih_plug::prelude::*;
use std::sync::Arc;

mod delay_line;
use delay_line::DelayLine;

mod oversampling;
use oversampling::HalfbandFilter;

const MAX_DELAY_TIME_SECONDS: f32 = 5.0;

pub struct Vibrato {
    params: Arc<VibratoParams>,
    delay_line: DelayLine,
    upsampler: HalfbandFilter,
    downsampler: HalfbandFilter,
}

#[derive(Params)]
struct VibratoParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "lfo-frequency"]
    pub lfo_frequency: FloatParam,

    #[id = "vibrato-width"]
    pub vibrato_width: FloatParam,
}

impl Default for Vibrato {
    fn default() -> Self {
        Self {
            params: Arc::new(VibratoParams::default()),
            delay_line: DelayLine::new(44100 * MAX_DELAY_TIME_SECONDS as usize),
            upsampler: HalfbandFilter::new(8, true),
            downsampler: HalfbandFilter::new(8, true),
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

            lfo_frequency: FloatParam::new(
                "LFO Frequency",
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

            vibrato_width: FloatParam::new(
                "Vibrato width",
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
        }
    }
}

impl Plugin for Vibrato {
    const NAME: &'static str = "Vibrato v0.0.7";
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
        // FIXME: the vibrato has a bit of aliasing... can we add oversampling maybe
        let sample_rate = _context.transport().sample_rate;
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();
            let lfo_frequency = self.params.lfo_frequency.smoothed.next();
            let vibrato_width = self.params.vibrato_width.smoothed.next();

            for sample in channel_samples {
                // Perform 4x oversampling
                let input = *sample;
                let mut frame = [input, 0., 0., 0.];

                for i in 0..frame.len() {
                    frame[i] = self.upsampler.process(frame[i]);
                    frame[i] = self.delay_line.process_with_vibrato(
                        *sample,
                        lfo_frequency,
                        vibrato_width,
                        sample_rate,
                    ) * gain;
                    frame[i] = self.downsampler.process(frame[i]);
                }

                *sample = frame[0];
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Vibrato {
    const CLAP_ID: &'static str = "https://renzomledesma.me";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A vibrato effect w/ wow & flutter");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Vibrato {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2__vibrato";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::PitchShift,
        Vst3SubCategory::Stereo,
    ];
}

// nih_export_clap!(Vibrato);
nih_export_vst3!(Vibrato);
