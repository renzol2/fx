use fx::moorer_verb::MoorerReverb;

/// Create a Moorer reverb instance with a given sample rate
///
/// The client is responsible for freeing the instance's memory when it's no longer required,
/// see `destroy()`.
#[no_mangle]
pub extern "C" fn create(sample_rate: usize) -> *mut MoorerReverb {
    Box::into_raw(Box::new(MoorerReverb::new(sample_rate)))
}

/// Destroy a Freeverb instance
///
/// # Safety
///
/// The instance must have been previously created using `create()`.
#[no_mangle]
pub unsafe extern "C" fn destroy(moorer_verb: *mut MoorerReverb) {
    if !moorer_verb.is_null() {
        unsafe {
            let _ = Box::from_raw(moorer_verb);
        }
    } else {
        panic!("")
    }
}

/// Process an audio buffer
///
/// # Safety
///
/// The input and output buffers must be (at least) sample_count f32s in size.
#[no_mangle]
pub unsafe extern "C" fn process(
    moorer_verb: &mut MoorerReverb,
    input_l: *const f32,
    input_r: *const f32,
    output_l: *mut f32,
    output_r: *mut f32,
    sample_count: usize,
) {
    for i in 0..sample_count as isize {
        let out = moorer_verb.tick((*input_l.offset(i), *input_r.offset(i)));
        *output_l.offset(i) = out.0 as f32;
        *output_r.offset(i) = out.1 as f32;
    }
}

#[no_mangle]
pub extern "C" fn set_damping(moorer_verb: &mut MoorerReverb, value: f32) {
    moorer_verb.set_damping(value);
}

#[no_mangle]
pub extern "C" fn set_frozen(moorer_verb: &mut MoorerReverb, value: bool) {
    moorer_verb.set_frozen(value);
}

#[no_mangle]
pub extern "C" fn set_wet(moorer_verb: &mut MoorerReverb, value: f32) {
    moorer_verb.set_wet(value);
}

#[no_mangle]
pub extern "C" fn set_width(moorer_verb: &mut MoorerReverb, value: f32) {
    moorer_verb.set_width(value);
}

#[no_mangle]
pub extern "C" fn set_room_size(moorer_verb: &mut MoorerReverb, value: f32) {
    moorer_verb.set_room_size(value);
}
