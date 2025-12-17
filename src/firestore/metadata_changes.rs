/// Controls whether metadata-only changes trigger snapshot events.
///
/// # C++ Equivalent
/// `firebase::firestore::MetadataChanges`
/// Reference: `firebase-cpp-sdk/firestore/src/include/firebase/firestore/metadata_changes.h:28`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataChanges {
    /// Listen to changes in metadata as well as data.
    ///
    /// Snapshot events will be triggered on metadata changes in addition to data changes.
    Include,

    /// Do not listen to metadata-only changes.
    ///
    /// Snapshot events will only be triggered when the document data changes.
    /// This is the default behavior.
    Exclude,
}

impl Default for MetadataChanges {
    fn default() -> Self {
        MetadataChanges::Exclude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_changes_default() {
        assert_eq!(MetadataChanges::default(), MetadataChanges::Exclude);
    }
}
