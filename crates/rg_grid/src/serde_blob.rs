use std::io::{Read, Write};
use std::marker::PhantomData;

use bytemuck::{CheckedBitPattern, NoUninit};
use serde::de::{Deserializer, Visitor};
use serde::ser::{Error, Serializer};

pub fn serialize<T, S>(data: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    T: NoUninit,
    S: Serializer,
{
    let blob = data_to_blob(data).ok_or_else(|| S::Error::custom("failed to write blob"))?;
    serializer.serialize_bytes(&blob)
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Box<[T]>, D::Error>
where
    T: CheckedBitPattern,
    D: Deserializer<'de>,
{
    struct BlobVisitor<T>(PhantomData<T>);

    impl<'de, T: CheckedBitPattern> Visitor<'de> for BlobVisitor<T> {
        type Value = Box<[T]>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a byte array")
        }

        fn visit_bytes<E>(self, blob: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            blob_to_data(blob).ok_or_else(|| E::custom("invalid blob"))
        }
    }

    deserializer.deserialize_bytes(BlobVisitor::<T>(PhantomData))
}

fn data_to_blob<T: NoUninit>(data: &[T]) -> Option<Vec<u8>> {
    let data_size = data.len() * std::mem::size_of::<T>();
    let mut blob = Vec::with_capacity(data_size);
    let mut encoder = zstd::Encoder::new(&mut blob, 0).ok()?;
    encoder.set_pledged_src_size(Some(data_size as u64)).ok()?;
    encoder.write_all(bytemuck::cast_slice(data)).ok()?;
    encoder.finish().ok()?;
    Some(blob)
}

fn blob_to_data<T: CheckedBitPattern>(blob: &[u8]) -> Option<Box<[T]>> {
    let mut uncompressed_data = Vec::new();
    let mut decoder = zstd::Decoder::new(blob).ok()?;
    decoder.read_to_end(&mut uncompressed_data).ok()?;

    let mut data = Vec::with_capacity(uncompressed_data.len() / std::mem::size_of::<T>());

    for bytes in uncompressed_data.chunks_exact(std::mem::size_of::<T>()) {
        data.push(bytemuck::checked::try_pod_read_unaligned(bytes).ok()?);
    }

    Some(data.into())
}
