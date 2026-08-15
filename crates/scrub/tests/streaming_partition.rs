use std::io::{self, Cursor, Read};
use std::path::Path;

use scrub::inspect_reader;

const READ_BOUNDARY: usize = 65_536;

struct PartitionedReader<'a> {
    input: &'a [u8],
    partitions: Vec<usize>,
    offset: usize,
    partition: usize,
}

impl<'a> PartitionedReader<'a> {
    fn new(input: &'a [u8], partitions: Vec<usize>) -> Self {
        assert!(partitions.iter().all(|size| *size > 0));
        assert_eq!(partitions.iter().sum::<usize>(), input.len());
        Self {
            input,
            partitions,
            offset: 0,
            partition: 0,
        }
    }
}

impl Read for PartitionedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.offset == self.input.len() {
            return Ok(0);
        }
        let partition_end = self.partitions[..=self.partition].iter().sum::<usize>();
        let count = (partition_end - self.offset).min(output.len());
        output[..count].copy_from_slice(&self.input[self.offset..self.offset + count]);
        self.offset += count;
        if self.offset == partition_end {
            self.partition += 1;
        }
        Ok(count)
    }
}

fn canonical(input: &[u8], partitions: Option<Vec<usize>>) -> Vec<u8> {
    let path = Path::new("partition-invariance.txt");
    let report = match partitions {
        Some(partitions) => inspect_reader(path, &mut PartitionedReader::new(input, partitions)),
        None => inspect_reader(path, &mut Cursor::new(input)),
    }
    .expect("inspection succeeds");
    report.canonical_report_bytes().expect("canonical bytes")
}

fn assert_partition(input: &[u8], partitions: Vec<usize>) {
    assert_eq!(canonical(input, Some(partitions)), canonical(input, None));
}

fn exhaustive_splits(input: &[u8]) {
    let expected = canonical(input, None);
    for split in 0..=input.len() {
        let partitions: Vec<_> = [split, input.len() - split]
            .into_iter()
            .filter(|size| *size > 0)
            .collect();
        let actual = if input.is_empty() {
            canonical(input, None)
        } else {
            canonical(input, Some(partitions))
        };
        assert_eq!(actual, expected, "split {split} of {}", input.len());
    }
}

fn seeded_partitions(length: usize, mut state: u64) -> Vec<usize> {
    let mut remaining = length;
    let mut output = Vec::new();
    while remaining > 0 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let ceiling = remaining.min(16_384);
        let size = 1 + usize::try_from(state % u64::try_from(ceiling).unwrap()).unwrap();
        output.push(size);
        remaining -= size;
    }
    output
}

#[test]
fn every_split_of_small_hostile_fixtures_is_semantically_identical() {
    let fixtures: &[&[u8]] = &[
        b"",
        b"ascii",
        "\u{00a2}\u{754c}\u{1f600}".as_bytes(),
        "A\u{202e}B\u{200b}e\u{301}\u{fb03}".as_bytes(),
        &[0xff],
        &[b'a', 0xe2, 0x82],
        &[b'a', 0xf0, 0x9f, 0x98, 0xff, b'z'],
    ];
    for fixture in fixtures {
        exhaustive_splits(fixture);
    }
}

#[test]
fn every_position_inside_representative_multibyte_scalars_is_invariant() {
    exhaustive_splits("x\u{00a2}y\u{754c}z\u{1f600}q".as_bytes());
}

#[test]
fn malformed_utf8_before_inside_after_and_late_is_invariant() {
    for input in [
        vec![0xff, b'a', b'b'],
        vec![b'a', 0xe2, 0x28, 0xa1, b'b'],
        vec![b'a', b'b', 0xf0, 0x9f, 0x92],
    ] {
        exhaustive_splits(&input);
    }

    let mut late = vec![b'a'; READ_BOUNDARY];
    late.push(0xff);
    for split in [READ_BOUNDARY - 1, READ_BOUNDARY, READ_BOUNDARY + 1] {
        let partitions = [split, late.len() - split]
            .into_iter()
            .filter(|size| *size > 0)
            .collect();
        assert_partition(&late, partitions);
    }
}

#[test]
fn exact_scanner_boundary_and_neighbor_lengths_are_invariant() {
    for length in [READ_BOUNDARY - 1, READ_BOUNDARY, READ_BOUNDARY + 1] {
        let input = vec![b'a'; length];
        for split in [1, length / 2, length.saturating_sub(1)] {
            let partitions: Vec<_> = [split, length - split]
                .into_iter()
                .filter(|size| *size > 0)
                .collect();
            assert_partition(&input, partitions);
        }
        assert_partition(&input, seeded_partitions(length, 0x5eed_b0a0_0000_0001));
    }
}

#[test]
fn unicode_evidence_types_crossing_the_real_buffer_boundary_are_invariant() {
    let mut input = vec![b'a'; READ_BOUNDARY - 1];
    input.extend_from_slice("\u{202e}\u{200b}e\u{301}\u{fb03}\u{1f600}".as_bytes());
    for split in READ_BOUNDARY - 3..=READ_BOUNDARY + 5 {
        assert_partition(&input, vec![split, input.len() - split]);
    }
    assert_partition(
        &input,
        vec![READ_BOUNDARY - 1, 1, input.len() - READ_BOUNDARY],
    );
}

#[test]
fn deterministic_seeded_partitions_cover_a_large_streamed_artifact() {
    let mut input = Vec::with_capacity(2 * 1024 * 1024);
    while input.len() < 2 * 1024 * 1024 {
        input.extend_from_slice("plain\u{200b}e\u{301}\u{202e}\u{2069}\u{1f600}\n".as_bytes());
    }
    input.truncate(2 * 1024 * 1024);
    for seed in [0x1, 0x5eed_b0a0, 0xd1ce_cafe_f00d_beef] {
        assert_partition(&input, seeded_partitions(input.len(), seed));
    }
}

#[test]
fn zero_length_artifact_uses_only_legal_eof_behavior() {
    assert_eq!(canonical(b"", Some(vec![])), canonical(b"", None));
}
