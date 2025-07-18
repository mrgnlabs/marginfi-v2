use anchor_lang::err;
use fixed::types::I80F48;
use marginfi_type_crate::types::{EmodeSettings, EMODE_ON};

use crate::{check, errors::MarginfiError, prelude::MarginfiResult};

pub trait EmodeSettingsImpl {
    fn validate_entries(&self) -> MarginfiResult;
    fn check_dupes(&self) -> MarginfiResult;
    fn is_enabled(&self) -> bool;
    fn update_emode_enabled(&mut self);
}

impl EmodeSettingsImpl for EmodeSettings {
    fn validate_entries(&self) -> MarginfiResult {
        for entry in self.emode_config.entries {
            if entry.is_empty() {
                continue;
            }
            let asset_init_w: I80F48 = I80F48::from(entry.asset_weight_init);
            let asset_maint_w: I80F48 = I80F48::from(entry.asset_weight_maint);

            check!(
                asset_init_w >= I80F48::ZERO && asset_init_w <= I80F48::ONE,
                MarginfiError::BadEmodeConfig
            );
            check!(
                asset_maint_w <= (I80F48::ONE + I80F48::ONE),
                MarginfiError::BadEmodeConfig
            );
            check!(asset_maint_w >= asset_init_w, MarginfiError::BadEmodeConfig);
        }

        // Validate that no duplicates exist (other than EMODE_TAG_EMPTY - 0)
        self.check_dupes()?;

        Ok(())
    }

    /// Note: expects entries to be sorted, invalid otherwise.
    fn check_dupes(&self) -> MarginfiResult {
        let non_empty_tags: Vec<u16> = self
            .emode_config
            .entries
            .iter()
            .filter(|e| !e.is_empty())
            .map(|e| e.collateral_bank_emode_tag)
            .collect();

        if non_empty_tags.windows(2).any(|w| w[0] == w[1]) {
            err!(MarginfiError::BadEmodeConfig)
        } else {
            Ok(())
        }
    }

    /// True if an emode configuration has been set (EMODE_ON)
    fn is_enabled(&self) -> bool {
        self.flags & EMODE_ON != 0
    }

    /// Sets EMODE on flag if configuration has any entries, removes the flag if it has no entries.
    fn update_emode_enabled(&mut self) {
        if self.emode_config.has_entries() {
            self.flags |= EMODE_ON;
        } else {
            self.flags &= !EMODE_ON;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    use fixed_macro::types::I80F48;
    use marginfi_type_crate::types::{
        reconcile_emode_configs, EmodeConfig, EmodeEntry, MAX_EMODE_ENTRIES,
    };

    fn create_entry(tag: u16, flags: u8, init: f32, maint: f32) -> EmodeEntry {
        EmodeEntry {
            collateral_bank_emode_tag: tag,
            flags,
            pad0: [0u8; 5],
            asset_weight_init: I80F48::from_num(init).into(),
            asset_weight_maint: I80F48::from_num(maint).into(),
        }
    }

    /// "Standard" entry with flags=0, init=0.7, maint=0.8.
    fn generic_entry(tag: u16) -> EmodeEntry {
        create_entry(tag, 0, 0.7, 0.8)
    }

    #[test]
    fn test_emode_valid_entries() {
        let mut settings = EmodeSettings::zeroed();
        settings.emode_config.entries[0] = generic_entry(1);
        settings.emode_config.entries[1] = generic_entry(2);
        settings.emode_config.entries[2] = generic_entry(3);
        // Note: The remaining entries stay zeroed (and are skipped during validation).
        assert!(settings.validate_entries().is_ok());
    }

    #[test]
    fn test_emode_invalid_duplicate_tags() {
        let mut settings = EmodeSettings::zeroed();
        settings.emode_config.entries[0] = generic_entry(1);
        settings.emode_config.entries[1] = generic_entry(1); // Duplicate tag: 1.
        settings.emode_config.entries[2] = generic_entry(2);
        let result = settings.validate_entries();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), MarginfiError::BadEmodeConfig.into());
    }

    #[test]
    fn test_emode_invalid_weight_too_high() {
        let mut settings = EmodeSettings::zeroed();
        // Using a weight greater than 1.0 is invalid.
        let entry = EmodeEntry {
            collateral_bank_emode_tag: 1,
            flags: 0,
            pad0: [0u8; 5],
            asset_weight_init: I80F48!(1.2).into(),
            asset_weight_maint: I80F48!(1.3).into(),
        };
        settings.emode_config.entries[0] = entry;
        let result = settings.validate_entries();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), MarginfiError::BadEmodeConfig.into());
    }

    #[test]
    fn test_emode_invalid_weight_main_le_init() {
        let mut settings = EmodeSettings::zeroed();
        // Maint must be greater than init
        let entry = EmodeEntry {
            collateral_bank_emode_tag: 1,
            flags: 0,
            pad0: [0u8; 5],
            asset_weight_init: I80F48!(0.8).into(),
            asset_weight_maint: I80F48!(0.7).into(),
        };
        settings.emode_config.entries[0] = entry;
        let result = settings.validate_entries();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), MarginfiError::BadEmodeConfig.into());
    }

    #[test]
    fn test_reconcile_emode_single_common_tag() {
        // Example 1:
        // * Config1 has an entry with tag 101, flags 1, init 0.7, maint 0.75.
        // * Config2 has an entry with tag 101, flags 0, init 0.6, maint 0.8.
        let entry1 = create_entry(101, 1, 0.7, 0.75);
        let entry2 = create_entry(101, 0, 0.6, 0.8);
        let config1 = EmodeConfig::from_entries(&[entry1]);
        let config2 = EmodeConfig::from_entries(&[entry2]);

        let reconciled = reconcile_emode_configs(vec![config1, config2]);

        // Expected: For tag 101, flags = min(1,0)=0, init = min(0.7,0.6)=0.6, maint = min(0.75,0.8)=0.75.
        let expected_entry = create_entry(101, 0, 0.6, 0.75);

        assert_eq!(reconciled.entries[0], expected_entry);
        // The rest of the entries should be zeroed.
        for entry in reconciled.entries.iter().skip(1) {
            assert!(entry.is_empty());
        }
    }

    #[test]
    fn test_reconcile_emode_no_common_tags() {
        // Example 2:
        // * Config1 has an entry with tag 99.
        // * Config2 has an entry with tag 101.
        // * Since there is no common tag across both, the result should be an empty (zeroed) config.
        let config1 = EmodeConfig::from_entries(&[generic_entry(99)]);
        let config2 = EmodeConfig::from_entries(&[generic_entry(101)]);

        let reconciled = reconcile_emode_configs(vec![config1, config2]);

        // Verify that all entries are empty.
        assert!(reconciled.entries.iter().all(|entry| entry.is_empty()));
    }

    #[test]
    fn test_reconcile_emode_multiple_configs() {
        // Example 3:
        // * Config1 has entries with tags 101 and 99.
        // * Config2 has an entry with tag 101.
        // * Config3 has an entry with tag 101.
        // * Only tag 101 is common to all configs.
        // * For tag 101:
        //   - Config1: flags 1, init 0.7, maint 0.75.
        //   - Config2: flags 0, init 0.6, maint 0.8.
        //   - Config3: flags 0, init 0.65, maint 0.8.
        // * The reconciled entry should have:
        //   - flags = min(1, 0, 0) = 0,
        //   - init   = min(0.7, 0.6, 0.65) = 0.6,
        //   - maint  = min(0.75, 0.8, 0.8) = 0.75.
        let entry1 = create_entry(101, 1, 0.7, 0.75);
        let entry2 = create_entry(101, 0, 0.6, 0.8);
        let entry3 = create_entry(101, 0, 0.65, 0.8);

        let config1 = EmodeConfig::from_entries(&[entry1, generic_entry(99)]);
        let config2 = EmodeConfig::from_entries(&[entry2]);
        let config3 = EmodeConfig::from_entries(&[entry3]);

        let reconciled = reconcile_emode_configs(vec![config1, config2, config3]);

        let expected_entry = create_entry(101, 0, 0.6, 0.75);

        assert_eq!(reconciled.entries[0], expected_entry);
        // All other entries should be zeroed.
        for entry in reconciled.entries.iter().skip(1) {
            assert!(entry.is_empty());
        }
    }

    #[test]
    #[should_panic(expected = "Too many EmodeEntry items")]
    fn test_emode_from_entries_panics_on_too_many_entries() {
        // Generate more entries than allowed.
        let mut entries = Vec::new();
        for i in 0..(MAX_EMODE_ENTRIES as u16 + 1) {
            entries.push(generic_entry(i));
        }
        // This call should panic.
        let _ = EmodeConfig::from_entries(&entries);
    }
}
