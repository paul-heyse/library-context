//! Receive decompression using explicitly owned scratch and output buffers.

use std::sync::Arc;
use bytes::{BufMut, BytesMut};
use crate::Status;
use super::{CompressionEncoding, ReceiveAdmission, ReceiveAllocation, ReceiveReservation};
use super::admission::ensure_capacity_for;

const OUTPUT_CHUNK: usize = 8192;

struct Output<'a> {
    bytes: &'a mut BytesMut,
    lease: &'a mut Option<Arc<dyn ReceiveReservation>>,
    owner: &'a dyn ReceiveAdmission,
    limit: usize,
}
impl Output<'_> {
    fn append(&mut self, bytes: &[u8]) -> Result<(), Status> {
        let required = self.bytes.len().checked_add(bytes.len()).ok_or_else(|| Status::resource_exhausted("Expanded receive size overflow"))?;
        if required > self.limit { return Err(Status::resource_exhausted("Expanded receive message exceeds its configured byte ceiling")); }
        if required > self.bytes.capacity() {
            let target = required.max(self.bytes.capacity().saturating_mul(2)).min(self.limit);
            ensure_capacity_for(self.bytes, self.lease, self.owner, ReceiveAllocation::DecompressedBuffer, target)?;
        }
        self.bytes.put_slice(bytes);
        Ok(())
    }
}

pub(crate) fn decompress(
    encoding: CompressionEncoding,
    input: &[u8],
    output: &mut BytesMut,
    lease: &mut Option<Arc<dyn ReceiveReservation>>,
    owner: &dyn ReceiveAdmission,
    limit: usize,
) -> Result<(), Status> {
    output.clear();
    let mut output = Output { bytes: output, lease, owner, limit };
    match encoding {
        #[cfg(feature = "gzip")]
        CompressionEncoding::Gzip => gzip(input, &mut output),
        #[cfg(feature = "deflate")]
        CompressionEncoding::Deflate => deflate(input, &mut output),
        #[cfg(feature = "zstd")]
        CompressionEncoding::Zstd => zstd(input, &mut output),
    }
}

#[cfg(any(feature = "gzip", feature = "deflate"))]
fn inflate(
    state: &mut miniz_oxide::inflate::stream::InflateState,
    input: &[u8],
    output: &mut Output<'_>,
    mut chunk_seen: impl FnMut(&[u8]),
) -> Result<usize, Status> {
    use miniz_oxide::{MZError, MZFlush, MZStatus};
    let mut consumed = 0usize;
    let mut chunk = [0u8; OUTPUT_CHUNK];
    loop {
        let result = miniz_oxide::inflate::stream::inflate(state, &input[consumed..], &mut chunk, MZFlush::None);
        consumed += result.bytes_consumed;
        output.append(&chunk[..result.bytes_written])?;
        chunk_seen(&chunk[..result.bytes_written]);
        match result.status {
            Ok(MZStatus::StreamEnd) => return Ok(consumed),
            Ok(_) | Err(MZError::Buf) if result.bytes_consumed != 0 || result.bytes_written != 0 => {},
            _ => return Err(Status::data_loss("Invalid or truncated compressed receive message")),
        }
    }
}

#[cfg(feature = "deflate")]
fn deflate(input: &[u8], output: &mut Output<'_>) -> Result<(), Status> {
    use miniz_oxide::{DataFormat, inflate::stream::InflateState};
    let _workspace = output.owner.reserve(ReceiveAllocation::CompressionWorkspace, std::mem::size_of::<InflateState>() + OUTPUT_CHUNK)?;
    let mut state = InflateState::new_boxed(DataFormat::Zlib);
    if inflate(&mut state, input, output, |_| {})? != input.len() { return Err(Status::data_loss("Trailing deflate receive bytes")); }
    Ok(())
}

#[cfg(feature = "gzip")]
fn gzip(input: &[u8], output: &mut Output<'_>) -> Result<(), Status> {
    use miniz_oxide::{DataFormat, inflate::stream::InflateState};
    let _workspace = output.owner.reserve(ReceiveAllocation::CompressionWorkspace, std::mem::size_of::<InflateState>() + OUTPUT_CHUNK + std::mem::size_of::<crc32fast::Hasher>())?;
    let mut state = InflateState::new_boxed(DataFormat::Raw);
    let mut offset = 0usize;
    while offset < input.len() {
        let start = offset;
        let header = input.get(offset..offset.saturating_add(10)).ok_or_else(|| Status::data_loss("Truncated gzip header"))?;
        if header[0..3] != [0x1f, 0x8b, 8] || header[3] & 0xe0 != 0 { return Err(Status::data_loss("Invalid gzip header")); }
        let flags = header[3]; offset += 10;
        if flags & 4 != 0 {
            let len = input.get(offset..offset.saturating_add(2)).ok_or_else(|| Status::data_loss("Truncated gzip extra length"))?;
            let len = u16::from_le_bytes([len[0], len[1]]) as usize;
            offset = offset.checked_add(2 + len).filter(|end| *end <= input.len()).ok_or_else(|| Status::data_loss("Truncated gzip extra field"))?;
        }
        for flag in [8, 16] {
            if flags & flag != 0 {
                let suffix = input.get(offset..).ok_or_else(|| Status::data_loss("Truncated gzip text field"))?;
                offset += suffix.iter().position(|byte| *byte == 0).ok_or_else(|| Status::data_loss("Unterminated gzip text field"))? + 1;
            }
        }
        if flags & 2 != 0 {
            let crc = input.get(offset..offset.saturating_add(2)).ok_or_else(|| Status::data_loss("Truncated gzip header checksum"))?;
            if crc32fast::hash(&input[start..offset]) as u16 != u16::from_le_bytes([crc[0], crc[1]]) { return Err(Status::data_loss("Invalid gzip header checksum")); }
            offset += 2;
        }
        state.reset(DataFormat::Raw);
        let mut crc = crc32fast::Hasher::new();
        let beginning = output.bytes.len();
        offset += inflate(&mut state, &input[offset..], output, |chunk| crc.update(chunk))?;
        let footer = input.get(offset..offset.saturating_add(8)).ok_or_else(|| Status::data_loss("Truncated gzip footer"))?;
        let expected_crc = u32::from_le_bytes(footer[..4].try_into().expect("four-byte checksum"));
        let expected_size = u32::from_le_bytes(footer[4..].try_into().expect("four-byte length"));
        if crc.finalize() != expected_crc || (output.bytes.len() - beginning) as u32 != expected_size { return Err(Status::data_loss("Invalid gzip receive checksum or length")); }
        offset += 8;
    }
    if offset == 0 { return Err(Status::data_loss("Empty gzip receive message")); }
    Ok(())
}

#[cfg(feature = "zstd")]
fn zstd(input: &[u8], output: &mut Output<'_>) -> Result<(), Status> {
    use std::ffi::c_void;
    use zstd_sys as sys;
    fn checked(value: usize) -> Result<usize, Status> {
        // SAFETY: ZSTD_isError accepts every size_t result and does not allocate.
        if unsafe { sys::ZSTD_isError(value) } != 0 { Err(Status::data_loss("Invalid zstd receive frame")) } else { Ok(value) }
    }
    let mut offset = 0usize;
    let mut workspace_size = 0usize;
    while offset < input.len() {
        let bytes = &input[offset..];
        // SAFETY: borrowed input pointer is valid for the supplied exact length;
        // these metadata functions only inspect bytes and never allocate.
        let frame_size = checked(unsafe { sys::ZSTD_findFrameCompressedSize(bytes.as_ptr().cast(), bytes.len()) })?;
        if frame_size == 0 || frame_size > bytes.len() { return Err(Status::data_loss("Invalid zstd receive frame size")); }
        let estimate = checked(unsafe { sys::ZSTD_estimateDStreamSize_fromFrame(bytes.as_ptr().cast(), frame_size) })?;
        workspace_size = workspace_size.max(estimate);
        offset += frame_size;
    }
    if workspace_size == 0 { return Err(Status::data_loss("Empty zstd receive message")); }
    #[repr(C, align(8))]
    #[derive(Clone)]
    struct Aligned(u64);
    let words = workspace_size.checked_add(7).ok_or_else(|| Status::resource_exhausted("Zstd workspace size overflow"))? / 8;
    let bytes = words.checked_mul(8).ok_or_else(|| Status::resource_exhausted("Zstd workspace size overflow"))?;
    let charge = bytes.checked_add(OUTPUT_CHUNK).ok_or_else(|| Status::resource_exhausted("Zstd scratch size overflow"))?;
    let _workspace = output.owner.reserve(ReceiveAllocation::CompressionWorkspace, charge)?;
    let mut workspace = vec![Aligned(0); words];
    // SAFETY: the vector is explicitly eight-byte aligned, covers the estimated
    // workspace, and outlives every decoder call. Static DStream never mallocs,
    // resizes or requires a native free; only this vector owns its storage.
    let context = unsafe { sys::ZSTD_initStaticDStream(workspace.as_mut_ptr().cast(), bytes) };
    if context.is_null() { return Err(Status::resource_exhausted("Zstd static workspace initialization failed")); }
    let mut source = sys::ZSTD_inBuffer { src: input.as_ptr().cast(), size: input.len(), pos: 0 };
    let mut chunk = [0u8; OUTPUT_CHUNK];
    loop {
        let before = source.pos;
        let mut destination = sys::ZSTD_outBuffer { dst: chunk.as_mut_ptr().cast::<c_void>(), size: chunk.len(), pos: 0 };
        // SAFETY: input/output positions stay within their explicit buffers and
        // the static context's aligned owner remains live throughout the call.
        let remaining = checked(unsafe { sys::ZSTD_decompressStream(context, &mut destination, &mut source) })?;
        output.append(&chunk[..destination.pos])?;
        if remaining == 0 && source.pos == source.size { return Ok(()); }
        if source.pos == before && destination.pos == 0 { return Err(Status::data_loss("Truncated zstd receive message")); }
    }
}
