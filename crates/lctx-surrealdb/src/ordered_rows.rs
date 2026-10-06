//! Attempt-owned external ordering of complete physical rows, with exact duplicate checking.
use lctx_model::domain::ModelError;
use std::{fs::File, io::{BufReader, BufWriter, Read, Write, Seek, SeekFrom}, path::Path};
use surrealdb::types::{RecordId, SurrealValue, Value};
use tempfile::{NamedTempFile, TempDir};

pub const ROW_BYTES: usize = 1024 * 1024;
const RUN_BYTES: usize = 1024 * 1024;
struct Entry { id: RecordId, bytes: Vec<u8> }
fn entry(value: Value) -> Result<Entry, ModelError> {
    let Value::Object(object) = &value else { return Err(ModelError::Schema("ordered physical row")); };
    let id = RecordId::from_value(object.get("id").ok_or(ModelError::Schema("ordered row identity"))?.clone()).map_err(ModelError::codec)?;
    let bytes = serde_json::to_vec(&value).map_err(ModelError::codec)?;
    if bytes.len() > ROW_BYTES { return Err(ModelError::Limit { owner: "native-ordered-rows", limit: "row bytes", observed: bytes.len(), bound: ROW_BYTES }); }
    Ok(Entry { id, bytes })
}
fn write(writer: &mut impl Write, entry: &Entry) -> Result<(), ModelError> {
    writer.write_all(&(entry.bytes.len() as u64).to_le_bytes()).map_err(ModelError::codec)?;
    writer.write_all(&entry.bytes).map_err(ModelError::codec)
}
fn read(reader: &mut impl Read) -> Result<Option<Entry>, ModelError> {
    let mut length = [0; 8];
    if reader.read(&mut length[..1]).map_err(ModelError::codec)? == 0 { return Ok(None); }
    reader.read_exact(&mut length[1..]).map_err(ModelError::codec)?;
    let length = usize::try_from(u64::from_le_bytes(length)).map_err(ModelError::codec)?;
    if length > ROW_BYTES { return Err(ModelError::Schema("ordered row frame bound")); }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(ModelError::codec)?;
    entry(serde_json::from_slice(&bytes).map_err(ModelError::codec)?).map(Some)
}
fn merge(directory: &Path, a: NamedTempFile, b: NamedTempFile) -> Result<NamedTempFile, ModelError> {
    let output = NamedTempFile::new_in(directory).map_err(ModelError::codec)?;
    let mut writer = BufWriter::new(output.reopen().map_err(ModelError::codec)?);
    let mut ar = BufReader::new(a.reopen().map_err(ModelError::codec)?);
    let mut br = BufReader::new(b.reopen().map_err(ModelError::codec)?);
    let mut av = read(&mut ar)?;
    let mut bv = read(&mut br)?;
    while av.is_some() || bv.is_some() {
        let take_a = match (&av, &bv) {
            (Some(a), Some(b)) if a.id == b.id => {
                if a.bytes != b.bytes { return Err(ModelError::Conflict("conflicting physical row identity")); }
                bv = read(&mut br)?;
                true
            }
            (Some(a), Some(b)) => a.id < b.id,
            (Some(_), None) => true,
            _ => false,
        };
        if take_a {
            write(&mut writer, av.as_ref().expect("merge source"))?;
            av = read(&mut ar)?;
        } else {
            write(&mut writer, bv.as_ref().expect("merge source"))?;
            bv = read(&mut br)?;
        }
    }
    writer.flush().map_err(ModelError::codec)?;
    Ok(output)
}

pub struct SortedRows {
    directory: TempDir,
    pending: Vec<Entry>,
    bytes: usize,
    limit: usize,
    // Binary carry merging keeps only logarithmically many run descriptors and two live heads.
    runs: Vec<Option<NamedTempFile>>,
}
impl SortedRows {
    pub fn new() -> Result<Self, ModelError> { Self::with_run_bytes(RUN_BYTES) }
    pub fn with_run_bytes(limit: usize) -> Result<Self, ModelError> {
        if limit == 0 || limit > RUN_BYTES { return Err(ModelError::Schema("external run byte limit")); }
        Ok(Self { directory: tempfile::tempdir().map_err(ModelError::codec)?, pending: Vec::new(), bytes: 0, limit, runs: Vec::new() })
    }
    pub fn push(&mut self, row: Value) -> Result<(), ModelError> {
        let row = entry(row)?;
        if self.bytes.saturating_add(row.bytes.len()) > self.limit { self.flush()?; }
        self.bytes += row.bytes.len();
        self.pending.push(row);
        if self.bytes >= self.limit { self.flush()?; }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), ModelError> {
        if self.pending.is_empty() { return Ok(()); }
        self.pending.sort_by(|a, b| a.id.cmp(&b.id));
        let mut last: Option<Entry> = None;
        let mut run = NamedTempFile::new_in(self.directory.path()).map_err(ModelError::codec)?;
        {
            let mut writer = BufWriter::new(run.as_file_mut());
            for row in self.pending.drain(..) {
                if let Some(prior) = &last {
                    if prior.id == row.id {
                        if prior.bytes != row.bytes { return Err(ModelError::Conflict("conflicting physical row identity")); }
                        continue;
                    }
                }
                write(&mut writer, &row)?;
                last = Some(row);
            }
            writer.flush().map_err(ModelError::codec)?;
        }
        self.bytes = 0;
        let mut level = 0;
        loop {
            if level == self.runs.len() { self.runs.push(None); }
            match self.runs[level].take() {
                Some(previous) => { run = merge(self.directory.path(), previous, run)?; level += 1; }
                None => { self.runs[level] = Some(run); break; }
            }
        }
        Ok(())
    }
    pub fn finish(mut self) -> Result<OrderedRows, ModelError> {
        self.flush()?;
        let mut final_run = None;
        for run in self.runs.into_iter().flatten() {
            final_run = Some(match final_run { Some(previous) => merge(self.directory.path(), previous, run)?, None => run });
        }
        let reader = final_run.as_ref().map(|run| run.reopen().map(BufReader::new)).transpose().map_err(ModelError::codec)?;
        Ok(OrderedRows { reader, _run: final_run, _directory: self.directory })
    }
}
pub struct OrderedRows {
    reader: Option<BufReader<File>>,
    _run: Option<NamedTempFile>,
    _directory: TempDir,
}
impl OrderedRows {
    pub fn rewind(&mut self) -> Result<(), ModelError> {
        if let Some(reader) = &mut self.reader { reader.seek(SeekFrom::Start(0)).map_err(ModelError::codec)?; }
        Ok(())
    }
    pub fn next(&mut self) -> Result<Option<Value>, ModelError> {
        let Some(reader) = &mut self.reader else { return Ok(None); };
        read(reader)?.map(|row| serde_json::from_slice(&row.bytes).map_err(ModelError::codec)).transpose()
    }
    /// Complete comparison preserves SDK value types, absent/null distinctions and float bits
    /// (including signed zero) instead of relying on numeric-equivalent Value equality.
    pub async fn reconcile(&mut self, actual: &mut crate::reader::NativeRows) -> Result<(), ModelError> {
        loop {
            match (self.next()?, actual.next().await?) {
                (None, None) => return Ok(()),
                (Some(expected), Some(actual)) if serde_json::to_vec(&expected).map_err(ModelError::codec)? == serde_json::to_vec(&actual).map_err(ModelError::codec)? => {}
                _ => return Err(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::{Bytes, Object};
    fn row(key: &str, value: Value) -> Value {
        let mut row = Object::new(); row.insert("id", RecordId::new("test", key)); row.insert("value", value); Value::Object(row)
    }
    #[test]
    fn external_runs_merge_full_values_and_deduplicate_across_runs() {
        let mut rows = SortedRows::with_run_bytes(128).unwrap();
        for n in (0..200).rev() {
            let value = row(&format!("{n:04}"), Value::Bytes(Bytes::from(vec![n as u8; 100])));
            rows.push(value.clone()).unwrap(); rows.push(value).unwrap();
        }
        let mut ordered = rows.finish().unwrap();
        for n in 0..200 {
            assert_eq!(ordered.next().unwrap(), Some(row(&format!("{n:04}"), Value::Bytes(Bytes::from(vec![n as u8; 100])))));
        }
        assert!(ordered.next().unwrap().is_none());
        ordered.rewind().unwrap(); assert!(ordered.next().unwrap().is_some());
    }
    #[test]
    fn sdk_serde_preserves_binary_record_ids_null_absence_and_signed_zero() {
        for value in [Value::None, Value::Null, Value::from_t(-0.0f64), Value::RecordId(RecordId::new("entity", "a")), Value::Bytes(Bytes::from(vec![0, 255]))] {
            let original = row("key", value);
            let bytes = serde_json::to_vec(&original).unwrap();
            let mut sorted = SortedRows::with_run_bytes(1).unwrap(); sorted.push(original).unwrap();
            assert_eq!(serde_json::to_vec(&sorted.finish().unwrap().next().unwrap().unwrap()).unwrap(), bytes);
        }
        let mut sorted = SortedRows::with_run_bytes(1).unwrap();
        sorted.push(row("same", Value::from_t(-0.0f64))).unwrap();
        assert!(sorted.push(row("same", Value::from_t(0.0f64))).is_err());
    }
    #[test]
    fn conflicting_duplicate_rows_and_oversized_values_are_refused() {
        let mut sorted = SortedRows::with_run_bytes(1).unwrap();
        sorted.push(row("same", Value::String("first".into()))).unwrap();
        assert!(sorted.push(row("same", Value::String("second".into()))).is_err());
        let mut sorted = SortedRows::new().unwrap();
        assert!(matches!(sorted.push(row("large", Value::String("x".repeat(ROW_BYTES)))), Err(ModelError::Limit { .. })));
    }
}
