pub fn multiply_volume(value: i32, volume: u8) -> i32 {
    (value * ((volume as i8) as i32)) >> 7
}

pub fn clamp(value: i32) -> i32 {
    if value < -32768 {
        return -32768;
    } else if value > 32767 {
        return 32767;
    }
    return value;
}

pub fn cast_arb_int(value: i32, bits: i32) -> i32 {
    let sign = 1i32 << (bits - 1);
    let mask = (sign << 1) - 1;

    ((value & mask) ^ sign) - sign
}
