#![no_main]

use std::io::{self, Cursor, Read};
use std::path::Path;

use libfuzzer_sys::fuzz_target;
use scrub::inspect_reader;

struct PartitionedReader<'a> {
    bytes: &'a [u8],
    partitions: Vec<usize>,
    offset: usize,
    partition: usize,
}

impl<'a> PartitionedReader<'a> {
    fn new(bytes: &'a [u8], partitions: Vec<usize>) -> Self {
        debug_assert!(partitions.iter().all(|size| *size > 0));
        debug_assert_eq!(partitions.iter().sum::<usize>(), bytes.len());
        Self {
            bytes,
            partitions,
            offset: 0,
            partition: 0,
        }
    }
}

impl Read for PartitionedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.offset == self.bytes.len() {
            return Ok(0);
        }
        let end = self.partitions[..=self.partition].iter().sum::<usize>();
        let count = (end - self.offset).min(output.len());
        output[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
        self.offset += count;
        if self.offset == end {
            self.partition += 1;
        }
        Ok(count)
    }
}

fn partitions(length: usize, description: &[u8]) -> Vec<usize> {
    let mut remaining = length;
    let mut output = Vec::new();
    let mut index = 0;
    while remaining > 0 {
        let described = description.get(index).copied().unwrap_or(255);
        let size = (usize::from(described) + 1).min(remaining);
        output.push(size);
        remaining -= size;
        index += 1;
    }
    output
}

fuzz_target!(|input: &[u8]| {
    let description_length = input.len().min(32);
    let (description, bytes) = input.split_at(description_length);
    let path = Path::new("fuzz-partition.txt");
    let one_shot = inspect_reader(path, &mut Cursor::new(bytes));
    let partitioned = inspect_reader(
        path,
        &mut PartitionedReader::new(bytes, partitions(bytes.len(), description)),
    );
    match (one_shot, partitioned) {
        (Ok(one_shot), Ok(partitioned)) => assert_eq!(
            one_shot.canonical_report_bytes().expect("canonical report"),
            partitioned
                .canonical_report_bytes()
                .expect("canonical report")
        ),
        (Err(one_shot), Err(partitioned)) => {
            assert_eq!(one_shot.to_string(), partitioned.to_string())
        }
        (one_shot, partitioned) => panic!(
            "reader partition changed success semantics: one-shot={}, partitioned={}",
            one_shot.is_ok(),
            partitioned.is_ok()
        ),
    }
});
