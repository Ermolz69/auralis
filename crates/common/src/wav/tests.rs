use super::*;
use std::io::Cursor;

fn wave(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let mut data = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
    data.extend_from_slice(&16_u32.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&rate.to_le_bytes());
    data.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    data.extend_from_slice(&(channels * 2).to_le_bytes());
    data.extend_from_slice(&16_u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(samples.len() as u32 * 2).to_le_bytes());
    for sample in samples {
        data.extend_from_slice(&sample.to_le_bytes());
    }
    let size = data.len() as u32 - 8;
    data[4..8].copy_from_slice(&size.to_le_bytes());
    data
}

#[test]
fn pcm_properties_include_exact_data_range_and_stereo_frame_count() {
    let bytes = wave(48000, 2, &[1, -2, 3, -4]);
    let properties = match inspect_pcm16(&mut Cursor::new(&bytes)) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(properties.sample_rate, 48000);
    assert_eq!(properties.channels, 2);
    assert_eq!(properties.frames, 2);
    assert_eq!(properties.data_offset, 44);
    assert_eq!(properties.data_bytes, 8);
    assert_eq!(
        &bytes[properties.data_offset as usize..],
        &[1, 0, 254, 255, 3, 0, 252, 255]
    );
}

#[test]
fn unknown_odd_chunks_are_skipped_with_their_padding() {
    let mut bytes = wave(24000, 1, &[7, 8]);
    bytes.splice(12..12, [b'J', b'U', b'N', b'K', 3, 0, 0, 0, 9, 8, 7, 0]);
    let size = bytes.len() as u32 - 8;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(matches!(
        inspect_pcm16(&mut Cursor::new(bytes)),
        Ok(WavProperties {
            data_offset: 56,
            frames: 2,
            ..
        })
    ));
}

#[test]
fn invalid_riff_lengths_truncation_and_trailing_bytes_fail() {
    let original = wave(48000, 1, &[1, 2]);
    for length in 0..original.len() {
        assert!(inspect_pcm16(&mut Cursor::new(&original[..length])).is_err());
    }
    let mut trailing = original;
    trailing.push(99);
    let size = trailing.len() as u32 - 8;
    trailing[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(inspect_pcm16(&mut Cursor::new(trailing)).is_err());
}

#[test]
fn invalid_pcm_fields_empty_data_and_partial_frames_fail() {
    let original = wave(48000, 2, &[1, 2]);
    for (offset, value) in [(20, 3), (22, 0), (28, 1), (32, 1), (34, 24)] {
        let mut bytes = original.clone();
        bytes[offset] = value;
        assert!(
            inspect_pcm16(&mut Cursor::new(bytes)).is_err(),
            "offset {offset}"
        );
    }
    assert!(inspect_pcm16(&mut Cursor::new(wave(48000, 1, &[]))).is_err());
    assert!(inspect_pcm16(&mut Cursor::new(wave(48000, 2, &[1]))).is_err());
    let mut huge_channels = original;
    huge_channels[22..24].copy_from_slice(&u16::MAX.to_le_bytes());
    huge_channels[32..34].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(inspect_pcm16(&mut Cursor::new(huge_channels)).is_err());
}

#[test]
fn duplicate_required_chunks_fail() {
    for kind in [b"fmt ", b"data"] {
        let mut bytes = wave(48000, 1, &[1]);
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        let size = bytes.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect_pcm16(&mut Cursor::new(bytes)).is_err());
    }
}

#[test]
fn millisecond_rounding_preserves_ties_to_even() {
    for (samples, duration) in [(vec![1], 0), (vec![1, 2, 3], 2)] {
        assert!(
            matches!(inspect_pcm16(&mut Cursor::new(wave(2000, 1, &samples))), Ok(properties) if properties.duration_ms == duration)
        );
    }
}

#[test]
fn excessive_chunk_metadata_is_bounded() {
    let mut bytes = wave(48000, 1, &[1]);
    let mut chunks = Vec::new();
    for _ in 0..4097 {
        chunks.extend_from_slice(b"JUNK\0\0\0\0");
    }
    bytes.splice(12..12, chunks);
    let size = bytes.len() as u32 - 8;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    assert!(matches!(
        inspect_pcm16(&mut Cursor::new(bytes)),
        Err(Pcm16Error::Invalid("WAV chunk count exceeds 4096"))
    ));
}
