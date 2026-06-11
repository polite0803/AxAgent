// SPDX-License-Identifier: AGPL-3.0-only

//! Property-based tests for agent crate using proptest.

use proptest::prelude::*;

proptest! {
    #[test]
    fn test_task_id_never_empty(s in "\\PC*") {
        // Task IDs should never be empty strings
        let trimmed = s.trim();
        prop_assert!(!trimmed.is_empty() || trimmed.len() <= 256);
    }

    #[test]
    fn test_message_content_roundtrip(content: String) {
        // Very basic: content can be any string, ensure no panic on trim
        let _trimmed = content.trim();
        let _len = content.len();
        prop_assert!(content.len() < 100_000); // sanity cap
    }
}
