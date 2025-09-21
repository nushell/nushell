use nu_engine::command_prelude::*;
use nu_protocol::Signals;
use rand::{
    Rng,
    distr::{Alphanumeric, StandardUniform},
    rng,
};

pub(super) enum RandomDistribution {
    Binary,
    Alphanumeric,
}

pub(super) fn random_byte_stream(
    distribution: RandomDistribution,
    length: usize,
    span: Span,
    signals: Signals,
) -> PipelineData {
    let stream_type = match distribution {
        RandomDistribution::Binary => ByteStreamType::Binary,
        RandomDistribution::Alphanumeric => ByteStreamType::String,
    };

    const OUTPUT_CHUNK_SIZE: usize = 8192;
    let mut remaining_bytes = length;
    PipelineData::byte_stream(
        ByteStream::from_fn(span, signals.clone(), stream_type, move |out| {
            if remaining_bytes == 0 || signals.interrupted() {
                return Ok(false);
            }

            let bytes_to_write = std::cmp::min(remaining_bytes, OUTPUT_CHUNK_SIZE);

            let rng = rng();
            let byte_iter: Box<dyn Iterator<Item = u8>> = match distribution {
                RandomDistribution::Binary => Box::new(rng.sample_iter(StandardUniform)),
                RandomDistribution::Alphanumeric => Box::new(rng.sample_iter(Alphanumeric)),
            };
            out.extend(byte_iter.take(bytes_to_write));

            remaining_bytes -= bytes_to_write;

            Ok(true)
        })
        .with_known_size(Some(length as u64)),
        None,
    )
    .with_span(span)
}
