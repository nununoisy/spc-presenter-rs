extern crate encoding_rs;

use self::encoding_rs::{SHIFT_JIS, WINDOWS_1252, CoderResult};
use std::io::{Result, ErrorKind, Error};
use std::str;

pub fn decode_string(s: &[u8]) -> Result<String> {
    let mut result = String::with_capacity(s.len() * 4);

    let mut cp1252_decoder = WINDOWS_1252.new_decoder();
    let (coder_result, _bytes_read, did_replacements) = cp1252_decoder.decode_to_string(s, &mut result, true);
    if coder_result != CoderResult::OutputFull && !did_replacements {
        return Ok(result);
    }

    result.clear();
    let mut shift_jis_decoder = SHIFT_JIS.new_decoder();
    let (coder_result, _bytes_read, did_replacements) = shift_jis_decoder.decode_to_string(s, &mut result, true);
    if coder_result != CoderResult::OutputFull && !did_replacements {
        return Ok(result);
    }

    match str::from_utf8(s) {
        Ok(s) => Ok(s.to_string()),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e))
    }
}
