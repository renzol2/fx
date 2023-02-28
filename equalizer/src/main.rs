use nih_plug::prelude::*;

use equalizer::Equalizer;

fn main() {
    nih_export_standalone::<Equalizer>();
}