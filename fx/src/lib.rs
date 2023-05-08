pub mod biquad;
pub mod dc_filter;
pub mod delay_line;
pub mod digital;
pub mod dynamics;
pub mod freeverb;
pub mod filters;
pub mod moorer_verb;
pub mod oversampling;
pub mod waveshapers;

pub const DEFAULT_SAMPLE_RATE: usize = 44_100;
pub const ABLETON_LIVE_MAX_BUFFER_SIZE: usize = 2048;
