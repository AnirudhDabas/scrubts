const MAX_INCOMPLETE_UTF8_LEN: usize = 3;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ScalarObservation {
    pub(crate) value: char,
    pub(crate) byte_offset: u64,
    pub(crate) scalar_offset: u64,
}

pub(crate) struct Decoder {
    incomplete: [u8; MAX_INCOMPLETE_UTF8_LEN],
    incomplete_len: usize,
    incomplete_byte_offset: u64,
    next_byte_offset: u64,
    scalar_offset: u64,
    invalid_utf8: bool,
}

impl Decoder {
    pub(crate) fn new() -> Self {
        Self {
            incomplete: [0; MAX_INCOMPLETE_UTF8_LEN],
            incomplete_len: 0,
            incomplete_byte_offset: 0,
            next_byte_offset: 0,
            scalar_offset: 0,
            invalid_utf8: false,
        }
    }

    pub(crate) fn push<E>(
        &mut self,
        bytes: &[u8],
        mut observe: impl FnMut(ScalarObservation) -> Result<(), E>,
    ) -> Result<(), PushError<E>> {
        if self.invalid_utf8 {
            return Ok(());
        }

        let chunk_byte_offset = self.next_byte_offset;
        self.next_byte_offset = self
            .next_byte_offset
            .checked_add(u64::try_from(bytes.len()).map_err(|_| PushError::OffsetOverflow)?)
            .ok_or(PushError::OffsetOverflow)?;

        let mut consumed = 0;
        if self.incomplete_len != 0 {
            consumed = self.complete_incomplete(bytes, &mut observe)?;
            if self.invalid_utf8 || self.incomplete_len != 0 {
                return Ok(());
            }
        }

        let remaining = &bytes[consumed..];
        let remaining_byte_offset = chunk_byte_offset
            .checked_add(u64::try_from(consumed).map_err(|_| PushError::OffsetOverflow)?)
            .ok_or(PushError::OffsetOverflow)?;
        match std::str::from_utf8(remaining) {
            Ok(text) => self.observe_valid_text(text, remaining_byte_offset, &mut observe),
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                let (valid, suffix) = remaining.split_at(valid_up_to);
                let text = std::str::from_utf8(valid)
                    .expect("Utf8Error::valid_up_to identifies a valid UTF-8 prefix");
                self.observe_valid_text(text, remaining_byte_offset, &mut observe)?;

                if error.error_len().is_some() {
                    self.invalid_utf8 = true;
                    return Ok(());
                }

                if suffix.len() > MAX_INCOMPLETE_UTF8_LEN {
                    self.invalid_utf8 = true;
                    return Ok(());
                }
                self.incomplete[..suffix.len()].copy_from_slice(suffix);
                self.incomplete_len = suffix.len();
                self.incomplete_byte_offset = remaining_byte_offset
                    .checked_add(u64::try_from(valid_up_to).map_err(|_| PushError::OffsetOverflow)?)
                    .ok_or(PushError::OffsetOverflow)?;
                Ok(())
            }
        }
    }

    pub(crate) fn finish(self) -> Result<(), InvalidUtf8> {
        if self.invalid_utf8 || self.incomplete_len != 0 {
            Err(InvalidUtf8)
        } else {
            Ok(())
        }
    }

    pub(crate) const fn is_potentially_valid(&self) -> bool {
        !self.invalid_utf8
    }

    fn complete_incomplete<E>(
        &mut self,
        bytes: &[u8],
        observe: &mut impl FnMut(ScalarObservation) -> Result<(), E>,
    ) -> Result<usize, PushError<E>> {
        let mut candidate = [0_u8; 4];
        let mut candidate_len = self.incomplete_len;
        candidate[..candidate_len].copy_from_slice(&self.incomplete[..candidate_len]);
        self.incomplete_len = 0;

        let mut consumed = 0;
        while consumed < bytes.len() {
            candidate[candidate_len] = bytes[consumed];
            candidate_len += 1;
            consumed += 1;

            match std::str::from_utf8(&candidate[..candidate_len]) {
                Ok(text) => {
                    self.observe_valid_text(text, self.incomplete_byte_offset, observe)?;
                    return Ok(consumed);
                }
                Err(error) if error.error_len().is_none() && candidate_len < 4 => {}
                Err(_) => {
                    self.invalid_utf8 = true;
                    return Ok(consumed);
                }
            }
        }

        debug_assert!(candidate_len <= MAX_INCOMPLETE_UTF8_LEN);
        self.incomplete[..candidate_len].copy_from_slice(&candidate[..candidate_len]);
        self.incomplete_len = candidate_len;
        Ok(consumed)
    }

    fn observe_valid_text<E>(
        &mut self,
        text: &str,
        byte_offset: u64,
        observe: &mut impl FnMut(ScalarObservation) -> Result<(), E>,
    ) -> Result<(), PushError<E>> {
        for (relative_byte_offset, value) in text.char_indices() {
            let relative_byte_offset =
                u64::try_from(relative_byte_offset).map_err(|_| PushError::OffsetOverflow)?;
            observe(ScalarObservation {
                value,
                byte_offset: byte_offset
                    .checked_add(relative_byte_offset)
                    .ok_or(PushError::OffsetOverflow)?,
                scalar_offset: self.scalar_offset,
            })
            .map_err(PushError::Observer)?;
            self.scalar_offset = self
                .scalar_offset
                .checked_add(1)
                .ok_or(PushError::OffsetOverflow)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PushError<E> {
    OffsetOverflow,
    Observer(E),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct InvalidUtf8;

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    #[test]
    fn scalar_observations_are_partition_invariant_with_exact_offsets() {
        let input = "A\u{00e9}\u{754c}\u{1f600}Z".as_bytes();
        let expected = vec![
            observation('A', 0, 0),
            observation('\u{00e9}', 1, 1),
            observation('\u{754c}', 3, 2),
            observation('\u{1f600}', 6, 3),
            observation('Z', 10, 4),
        ];

        assert_eq!(decode_by_chunk_size(input, input.len()), expected);
        for chunk_size in [1, 2, 3, 7] {
            assert_eq!(decode_by_chunk_size(input, chunk_size), expected);
        }
        assert_eq!(decode_by_partition(input, &[2, 2, 3, 1, 1, 2]), expected);

        for _ in 0..8 {
            assert_eq!(decode_by_partition(input, &[2, 2, 3, 1, 1, 2]), expected);
        }
    }

    #[test]
    fn short_chunks_and_the_inspection_boundary_produce_the_same_stream() {
        let mut input = vec![b'a'; 65_535];
        input.extend_from_slice("\u{754c}\u{1f600}z".as_bytes());

        let expected = decode_by_chunk_size(&input, input.len());
        assert_eq!(decode_by_chunk_size(&input, 1), expected);
        assert_eq!(decode_by_chunk_size(&input, 2), expected);
        assert_eq!(
            decode_by_partition(&input, &[65_536, input.len() - 65_536]),
            expected
        );
        assert_eq!(expected[65_535], observation('\u{754c}', 65_535, 65_535));
        assert_eq!(expected[65_536], observation('\u{1f600}', 65_538, 65_536));
    }

    #[test]
    fn incomplete_suffix_carries_every_legal_scalar_split() {
        for value in ['\u{00e9}', '\u{754c}', '\u{1f600}'] {
            let encoded = value.to_string().into_bytes();
            for split in 1..encoded.len() {
                let mut decoder = Decoder::new();
                let mut observations = Vec::new();

                push(&mut decoder, &encoded[..split], &mut observations);
                assert!(!decoder.invalid_utf8);
                assert_eq!(decoder.incomplete_len, split);
                assert!(decoder.incomplete_len <= MAX_INCOMPLETE_UTF8_LEN);
                assert!(observations.is_empty());

                push(&mut decoder, &encoded[split..], &mut observations);
                assert_eq!(decoder.incomplete_len, 0);
                assert_eq!(observations, [observation(value, 0, 0)]);
                assert_eq!(decoder.finish(), Ok(()));
            }
        }
    }

    #[test]
    fn definite_malformed_utf8_is_not_an_incomplete_suffix() {
        for malformed in [&[0x80][..], &[0xc0, 0xaf][..], &[0xed, 0xa0, 0x80][..]] {
            let mut decoder = Decoder::new();
            let mut observations = Vec::new();
            push(&mut decoder, malformed, &mut observations);

            assert!(decoder.invalid_utf8);
            assert_eq!(decoder.incomplete_len, 0);
            assert_eq!(decoder.finish(), Err(InvalidUtf8));
        }

        let mut incomplete = Decoder::new();
        let mut observations = Vec::new();
        push(&mut incomplete, &[0xe2, 0x80], &mut observations);
        assert!(!incomplete.invalid_utf8);
        assert_eq!(incomplete.incomplete_len, 2);
        assert_eq!(incomplete.finish(), Err(InvalidUtf8));
    }

    #[test]
    fn malformed_sequence_can_become_definite_after_a_chunk_split() {
        let mut decoder = Decoder::new();
        let mut observations = Vec::new();
        push(&mut decoder, &[0xe0], &mut observations);
        assert!(!decoder.invalid_utf8);
        assert_eq!(decoder.incomplete_len, 1);

        push(&mut decoder, &[0x80], &mut observations);
        assert!(decoder.invalid_utf8);
        assert_eq!(decoder.incomplete_len, 0);
        assert_eq!(decoder.finish(), Err(InvalidUtf8));
    }

    #[test]
    fn incomplete_eof_invalidates_after_emitting_only_the_valid_prefix() {
        let mut decoder = Decoder::new();
        let mut observations = Vec::new();
        push(&mut decoder, &[b'A', 0xe2, 0x80], &mut observations);

        assert_eq!(observations, [observation('A', 0, 0)]);
        assert!(!decoder.invalid_utf8);
        assert_eq!(decoder.incomplete_len, 2);
        assert_eq!(decoder.finish(), Err(InvalidUtf8));
    }

    fn observation(value: char, byte_offset: u64, scalar_offset: u64) -> ScalarObservation {
        ScalarObservation {
            value,
            byte_offset,
            scalar_offset,
        }
    }

    fn decode_by_chunk_size(input: &[u8], chunk_size: usize) -> Vec<ScalarObservation> {
        let partition: Vec<_> = input.chunks(chunk_size).map(<[u8]>::len).collect();
        decode_by_partition(input, &partition)
    }

    fn decode_by_partition(input: &[u8], partition: &[usize]) -> Vec<ScalarObservation> {
        assert_eq!(partition.iter().sum::<usize>(), input.len());
        let mut decoder = Decoder::new();
        let mut observations = Vec::new();
        let mut start = 0;
        for &length in partition {
            let end = start + length;
            push(&mut decoder, &input[start..end], &mut observations);
            assert!(decoder.incomplete_len <= MAX_INCOMPLETE_UTF8_LEN);
            start = end;
        }
        assert_eq!(decoder.finish(), Ok(()));
        observations
    }

    fn push(decoder: &mut Decoder, bytes: &[u8], observations: &mut Vec<ScalarObservation>) {
        decoder
            .push(bytes, |observation| {
                observations.push(observation);
                Ok::<(), Infallible>(())
            })
            .expect("test offsets do not overflow");
    }
}
