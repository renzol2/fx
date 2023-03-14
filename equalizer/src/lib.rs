use std::f32::consts::PI;
use std::sync::Arc;
use std::{sync::atomic::AtomicBool};


use nih_plug::prelude::*;

enum BiquadFilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    AllPass,
    ParametricEQ,
    LowShelf,
    HighShelf,
}

// Biquad filter code from: https://www.earlevel.com/main/2012/11/26/biquad-c-source-code/
struct BiquadFilter {
    // Filter type & coefficients
    filter_type: BiquadFilterType,
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,

    // Filter parameters
    fc: f32,
    q: f32,
    peak_gain: f32,

    // Unit delays
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    fn new() -> BiquadFilter {
        let mut bqf = BiquadFilter {
            filter_type: BiquadFilterType::LowPass,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            fc: 0.5,
            q: 0.707,
            peak_gain: 0.0,
            z1: 0.0,
            z2: 0.0,
        };
        bqf.set_filter_type(BiquadFilterType::LowPass);
        bqf
    }

    fn set_filter_type(&mut self, filter_type: BiquadFilterType) {
        self.filter_type = filter_type;
        self.calculate_biquad_coefficients();
    }

    fn set_q(&mut self, q: f32) {
        self.q = q;
        self.calculate_biquad_coefficients();
    }

    fn set_fc(&mut self, fc: f32) {
        self.fc = fc;
        self.calculate_biquad_coefficients();
    }

    fn set_peak_gain(&mut self, peak_gain: f32) {
        self.peak_gain = peak_gain;
        self.calculate_biquad_coefficients();
    }

    fn set_biquad(&mut self, filter_type: BiquadFilterType, fc: f32, q: f32, peak_gain: f32) {
        self.filter_type = filter_type;
        self.q = q;
        self.fc = fc;
        self.set_peak_gain(peak_gain)
    }

    fn calculate_biquad_coefficients(&mut self) {
        let v = 10.0_f32.powf(self.peak_gain.abs() / 20.0);
        let k = (PI * self.fc).tan();

        // TODO: make this a match statement with filter types
        // For now, default to LPF
        let norm = (1.0 + k / self.q + k * k).recip();
        self.a0 = k * k * norm;
        self.a1 = 2.0 * self.a0;
        self.a2 = self.a0;
        self.b1 = 2.0 * (k * k - 1.0) * norm;
        self.b2 = (1.0 - k / self.q + k * k) * norm;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = input * self.a0 + self.z1;
        self.z1 = input * self.a1 + self.z2 - self.b1 * output;
        self.z2 = input * self.a2 - self.b2 * output;
        output
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
        let should_update_filter = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let params = Arc::new(EqualizerParams::new(should_update_filter.clone()));
        Self {
            params,
            should_update_filter,
            biquad: BiquadFilter::new(),
        }
    }
}

impl EqualizerParams {
    fn new(should_update_filter: Arc<AtomicBool>) -> Self {
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
                    factor: FloatRange::skew_factor(-2.2),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_callback(Arc::new({
                let should_update_filter = should_update_filter.clone();
                move |_| should_update_filter.store(true, std::sync::atomic::Ordering::SeqCst)
            })),

            q: FloatParam::new(
                "Q",
                1.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 18.0,
                    factor: FloatRange::skew_factor(-2.2),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_callback(Arc::new({
                let should_update_filter = should_update_filter.clone();
                move |_| should_update_filter.store(true, std::sync::atomic::Ordering::SeqCst)
            })),
        }
    }
}

impl Plugin for Equalizer {
    const NAME: &'static str = "Equalizer v0.0.2.9";
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
        // FIXME: this only updating once when changing the frequency; after updating once, it no longer updates...
        if self
            .should_update_filter
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            let frequency = self.params.cutoff_frequency.smoothed.next();
            let q = self.params.q.smoothed.next();

            let sample_rate = _context.transport().sample_rate;
            let fc = frequency / sample_rate;
            self.biquad.set_biquad(BiquadFilterType::LowPass, fc, q, 0.0);
            self.biquad.calculate_biquad_coefficients();
        }

        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                let processed = self.biquad.process(*sample);
                *sample = processed * gain;
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
