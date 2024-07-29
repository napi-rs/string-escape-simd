use std::{
    ops::BitOr,
    simd::{
        cmp::{SimdPartialEq, SimdPartialOrd},
        Simd,
    },
};

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE};

pub fn encode_str<S: AsRef<str>>(input: S, output: &mut String) {
    let writer = unsafe { output.as_mut_vec() };
    writer.push(b'"');

    let input = input.as_ref();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let quote_mask = Simd::splat(b'"');
    let reverse_solidus_mask = Simd::splat(b'\\');
    let solidus_mask = Simd::splat(b'/');
    let bs_mask = Simd::splat(b'\x08');
    let ff_mask = Simd::splat(b'\x0C');
    let lf_mask = Simd::splat(b'\n');
    let cr_mask = Simd::splat(b'\r');
    let tab_mask = Simd::splat(b'\t');
    let control_mask = Simd::splat(0x1F);

    while i + 32 <= len {
        let next_chunk = i + 32;
        let chunks = &bytes[i..next_chunk];
        let simd: Simd<u8, 32> = Simd::from_slice(chunks);
        let mask = simd
            .simd_le(control_mask)
            .bitor(simd.simd_eq(quote_mask))
            .bitor(simd.simd_eq(reverse_solidus_mask))
            .bitor(simd.simd_eq(solidus_mask))
            .bitor(simd.simd_eq(bs_mask))
            .bitor(simd.simd_eq(ff_mask))
            .bitor(simd.simd_eq(lf_mask))
            .bitor(simd.simd_eq(cr_mask))
            .bitor(simd.simd_eq(tab_mask));

        if !mask.any() {
            writer.extend_from_slice(chunks);
            i += 32;
            continue;
        }

        for (byte_position, is_backslash) in mask.to_array().into_iter().enumerate() {
            if !is_backslash {
                writer.push(chunks[byte_position]);
            } else {
                let c = chunks[byte_position];
                let escape = ESCAPE[c as usize];
                if escape == 0 {
                    continue;
                }

                let char_escape = CharEscape::from_escape_table(escape, c as u8);
                write_char_escape(writer, char_escape);
            }
        }
        i += 32;
    }

    if i < len {
        encode_str_inner(&input[i..], writer);
    }

    output.push('"');
}
