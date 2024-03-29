use nih_plug::prelude::*;
use std::sync::Arc;

use fx::{
    biquad::{BiquadFilterType, StereoBiquadFilter},
    dc_filter::DcFilter,
    oversampling::HalfbandFilter,
    waveshapers::*,
    DEFAULT_SAMPLE_RATE,
};

/// Distortion algorithms available in plugin
#[derive(Enum, Debug, PartialEq, Eq)]
pub enum DistortionType {
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
    #[name = "Diode rectifier"]
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

/// Process input sample through waveshaper algorithm of specified type
pub fn distort_sample(distortion_type: &DistortionType, drive: f32, input_sample: f32) -> f32 {
    match distortion_type {
        DistortionType::Saturation => get_saturator_output(drive, input_sample),
        DistortionType::HardClipping => get_hard_clipper_output(drive, input_sample),
        DistortionType::FuzzyRectifier => get_fuzzy_rectifier_output(drive, input_sample),
        DistortionType::ShockleyDiodeRectifier => {
            get_shockley_diode_rectifier_output(drive, input_sample)
        }
        DistortionType::Dropout => get_dropout_output(drive, input_sample),
        DistortionType::DoubleSoftClipper => get_double_soft_clipper_output(drive, input_sample),
        DistortionType::Wavefolding => get_wavefolder_output(drive, input_sample),
    }
}

const FILTER_CUTOFF_HZ: f32 = 8000.0;
const OVERSAMPLING_FACTOR: usize = 4;

pub struct Distortion {
    params: Arc<DistortionParams>,
    upsampler: (HalfbandFilter, HalfbandFilter),
    downsampler: (HalfbandFilter, HalfbandFilter),
    prefilter: StereoBiquadFilter,
    postfilter: StereoBiquadFilter,
    dc_filters: (DcFilter, DcFilter),
    oversample_factor: usize,
}

#[derive(Params)]
struct DistortionParams {
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

    #[id = "enable-pre-filter"]
    pub enable_pre_filter: BoolParam,

    #[id = "enable-post-filter"]
    pub enable_post_filter: BoolParam,
}

impl Default for Distortion {
    fn default() -> Self {
        // Setup filters
        let mut prefilter = StereoBiquadFilter::new();
        let mut postfilter = StereoBiquadFilter::new();

        // Biquad parameters tuned by ear
        let fc = FILTER_CUTOFF_HZ / DEFAULT_SAMPLE_RATE as f32; // hz, using default sample rate
        let gain = 18.0; // dB
        let q = 0.1;
        prefilter.set_biquads(BiquadFilterType::HighShelf, fc, q, gain);
        postfilter.set_biquads(BiquadFilterType::LowShelf, fc, q, -gain);

        Distortion {
            params: Arc::new(DistortionParams::default()),
            upsampler: (HalfbandFilter::new(8, true), HalfbandFilter::new(8, true)),
            downsampler: (HalfbandFilter::new(8, true), HalfbandFilter::new(8, true)),
            prefilter,
            postfilter,
            dc_filters: (DcFilter::default(), DcFilter::default()),
            oversample_factor: 4,
        }
    }
}

impl Default for DistortionParams {
    fn default() -> Self {
        Self {
            input_gain: FloatParam::new(
                "Input Gain",
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
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            distortion_type: EnumParam::new("Type", DistortionType::Saturation),

            enable_pre_filter: BoolParam::new("Enable pre-filter", true),

            enable_post_filter: BoolParam::new("Enable post-filter", true),
        }
    }
}

impl Plugin for Distortion {
    const NAME: &'static str = "Distortion v0.1.4";
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
        let fs = _buffer_config.sample_rate;
        if fs >= 88200. {
            self.oversample_factor = 1;
        } else {
            self.oversample_factor = 4;
        }

        self.prefilter.set_fc(FILTER_CUTOFF_HZ / fs);
        self.postfilter.set_fc(FILTER_CUTOFF_HZ / fs);

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
            let input_gain = self.params.input_gain.smoothed.next();
            let output_gain = self.params.output_gain.smoothed.next();
            let drive = self.params.drive.smoothed.next();
            let dry_wet_ratio = self.params.dry_wet_ratio.smoothed.next();
            let distortion_type = self.params.distortion_type.value();
            let enable_pre_filter = self.params.enable_pre_filter.value();
            let enable_post_filter = self.params.enable_post_filter.value();

            let in_l = *channel_samples.get_mut(0).unwrap();
            let in_r = *channel_samples.get_mut(1).unwrap();

            let processed_l = self.dc_filters.0.process(in_l) * input_gain;
            let processed_r = self.dc_filters.1.process(in_r) * input_gain;

            let (wet_l, wet_r) = if self.oversample_factor == OVERSAMPLING_FACTOR {
                // Begin upsampling block
                let mut frame_l = [processed_l, 0., 0., 0.];
                let mut frame_r = [processed_r, 0., 0., 0.];

                for i in 0..OVERSAMPLING_FACTOR {
                    // Upsample
                    frame_l[i] = self.upsampler.0.process(frame_l[i]);
                    frame_r[i] = self.upsampler.1.process(frame_r[i]);

                    // Apply pre-filtering
                    if enable_pre_filter {
                        let prefiltered = self.prefilter.process((frame_l[i], frame_r[i]));
                        frame_l[i] = prefiltered.0;
                        frame_r[i] = prefiltered.1;
                    }

                    // Apply distortion
                    frame_l[i] = distort_sample(&distortion_type, drive, frame_l[i]);
                    frame_r[i] = distort_sample(&distortion_type, drive, frame_r[i]);

                    // Apply post-filtering
                    if enable_post_filter {
                        let postfiltered = self.postfilter.process((frame_l[i], frame_r[i]));
                        frame_l[i] = postfiltered.0;
                        frame_r[i] = postfiltered.1;
                    }

                    // Downsample through half-band filter
                    frame_l[i] = self.downsampler.0.process(frame_l[i]);
                    frame_r[i] = self.downsampler.1.process(frame_r[i]);
                }

                (frame_l[0], frame_r[0])
            } else {
                let distorted_l = distort_sample(&distortion_type, drive, processed_l);
                let distorted_r = distort_sample(&distortion_type, drive, processed_r);
                (distorted_l, distorted_r)
            };

            let out_l = (in_l * (1.0 - dry_wet_ratio)) + (wet_l * dry_wet_ratio);
            let out_r = (in_r * (1.0 - dry_wet_ratio)) + (wet_r * dry_wet_ratio);

            *channel_samples.get_mut(0).unwrap() = out_l * output_gain;
            *channel_samples.get_mut(1).unwrap() = out_r * output_gain;
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

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Distortion,
    ];
}

impl Vst3Plugin for Distortion {
    const VST3_CLASS_ID: [u8; 16] = *b"renzol2_distortn";
    const VST3_CATEGORIES: &'static str = "Fx|Distortion";
}

nih_export_vst3!(Distortion);
