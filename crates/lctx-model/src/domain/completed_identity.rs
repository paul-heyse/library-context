//! Complete logical identity is independent of physical installation authority.
use super::{ContentHash, Key, KeySink, ModelError};
use serde::{Deserialize, Serialize};

use super::recovery_closure::{COMPLETED_FAMILIES, RecoveryFamily as CompletedFamily};
fn family_tag(family: CompletedFamily) -> Result<&'static [u8], ModelError> {
    match family {
        CompletedFamily::Contributions => Ok(b"contributions"),
        CompletedFamily::Memberships => Ok(b"memberships"),
        CompletedFamily::Views => Ok(b"views"),
        CompletedFamily::TypedBacking => Ok(b"typed-backing"),
        CompletedFamily::Bindings => Ok(b"bindings"),
        CompletedFamily::Aliases => Ok(b"aliases"),
        _ => Err(ModelError::Conflict("not a completed logical family")),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedFamilyRoot {
    pub family: CompletedFamily,
    pub rows: u64,
    pub content: ContentHash,
}
/// This computes semantic identity only. Possession never grants native completion,
/// physical validity, admission, or a right to skip independent cold checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedFamilyRoots {
    roots: [CompletedFamilyRoot; 6],
}
impl CompletedFamilyRoots {
    pub fn new(roots: [CompletedFamilyRoot; 6]) -> Result<Self, ModelError> {
        if roots
            .iter()
            .zip(COMPLETED_FAMILIES)
            .any(|(root, family)| root.family != family)
        {
            return Err(ModelError::Conflict(
                "completed logical family inventory/order",
            ));
        }
        Ok(Self { roots })
    }
    pub fn roots(&self) -> &[CompletedFamilyRoot; 6] {
        &self.roots
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        Self::new(self.roots.clone())?;
        let mut sink = KeySink::new("native-completed-state/v4");
        for root in &self.roots {
            sink.part(b"family", family_tag(root.family)?);
            sink.part(b"rows", &root.rows.to_le_bytes());
            root.content.encode(&mut sink);
        }
        Ok(sink.finish())
    }
}
pub struct CompletedFamilyHasher {
    family: CompletedFamily,
    rows: u64,
    sink: KeySink,
}
impl CompletedFamilyHasher {
    pub fn new(family: CompletedFamily) -> Result<Self, ModelError> {
        let mut sink = KeySink::new("native-completed-family/v1");
        sink.part(b"family", family_tag(family)?);
        Ok(Self {
            family,
            rows: 0,
            sink,
        })
    }
    /// The owner supplies the complete canonical logical row stream in its declared
    /// deterministic order, including explicit empties/selectors/absence semantics.
    /// Actual membership/backing and duplicate validation remain the owner's job.
    pub fn row(&mut self, canonical: &[u8]) -> Result<(), ModelError> {
        self.rows = self
            .rows
            .checked_add(1)
            .ok_or(ModelError::Schema("completed family count"))?;
        self.sink.part(b"row", canonical);
        Ok(())
    }
    pub fn finish(mut self) -> CompletedFamilyRoot {
        self.sink.part(b"rows", &self.rows.to_le_bytes());
        CompletedFamilyRoot {
            family: self.family,
            rows: self.rows,
            content: self.sink.finish(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn roots() -> CompletedFamilyRoots {
        CompletedFamilyRoots::new(
            COMPLETED_FAMILIES.map(|family| CompletedFamilyHasher::new(family).unwrap().finish()),
        )
        .unwrap()
    }
    #[test]
    fn every_family_and_explicit_empty_inventory_changes_complete_identity() {
        let empty = roots();
        let identity = empty.identity().unwrap();
        for index in 0..6 {
            let mut changed = empty.roots.clone();
            let mut family = CompletedFamilyHasher::new(COMPLETED_FAMILIES[index]).unwrap();
            family.row(b"complete canonical logical row").unwrap();
            changed[index] = family.finish();
            assert_ne!(
                CompletedFamilyRoots::new(changed)
                    .unwrap()
                    .identity()
                    .unwrap(),
                identity
            );
        }
        let mut wrong = empty.roots.clone();
        wrong.swap(0, 1);
        assert!(CompletedFamilyRoots::new(wrong).is_err());
        let mut wrong = empty.roots.clone();
        wrong[5] = wrong[4].clone();
        assert!(CompletedFamilyRoots::new(wrong).is_err());
    }
    #[test]
    fn row_order_count_and_full_content_are_identity_bearing() {
        let root = |rows: &[&[u8]]| {
            let mut family = CompletedFamilyHasher::new(CompletedFamily::Memberships).unwrap();
            for row in rows {
                family.row(row).unwrap();
            }
            family.finish()
        };
        let canonical = root(&[b"key-a/value-a", b"key-b/value-b"]);
        assert_eq!(canonical, root(&[b"key-a/value-a", b"key-b/value-b"]));
        assert_ne!(canonical, root(&[b"key-b/value-b", b"key-a/value-a"]));
        assert_ne!(canonical, root(&[b"key-a/value-a", b"key-b/changed"]));
        assert_ne!(canonical, root(&[b"key-a/value-a"]));
    }
}
