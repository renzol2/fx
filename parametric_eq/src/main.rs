use nih_plug::prelude::*;

use parametric_eq::ParametricEq;

fn main() {
    nih_export_standalone::<ParametricEq>();
}