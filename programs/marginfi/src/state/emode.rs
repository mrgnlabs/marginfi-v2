use super::marginfi_group::WrappedI80F48;
use crate::{
    assert_struct_align, assert_struct_size, check, errors::MarginfiError, prelude::MarginfiResult,
};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use fixed::types::I80F48;
use type_layout::TypeLayout;

pub const EMODE_ON: u64 = 1;

pub const MAX_EMODE_ENTRIES: usize = 10;
pub const EMODE_TAG_EMPTY: u16 = 0;

assert_struct_size!(EmodeSettings, 424);
assert_struct_align!(EmodeSettings, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout, Debug,
)]
/// Controls the bank's e-mode configuration, allowing certain collateral sources to be treated more
/// favorably as collateral when used to borrow from this bank.
pub struct EmodeSettings {
    /// This bank's NON-unique id that other banks will use to determine what emode rate to use when
    /// this bank is offered as collateral.
    ///
    /// For example, all stablecoin banks might share the same emode_tag, and in their entries, each
    /// such stablecoin bank will recognize that collateral sources with this "stable" tag get
    /// preferrential weights. When a new stablecoin is added that is considered riskier, it may get
    /// a new, less favorable emode tag, and eventually get ugraded to the same one as the other
    /// stables
    ///
    /// * 0 is in an invalid tag and will do nothing.
    pub emode_tag: u16,
    // To next 8-byte multiple
    pub pad0: [u8; 6],

    /// Unix timestamp from the system clock when emode state was last updated
    pub timestamp: i64,
    /// EMODE_ON (1) - If set, at least one entry is configured
    /// 2, 4, 8, etc, Reserved for future use
    pub flags: u64,
    /// This bank's emode configurations.
    pub entries: [EmodeEntry; MAX_EMODE_ENTRIES],
}

impl Default for EmodeSettings {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl EmodeSettings {
    pub fn validate_entries(&self) -> MarginfiResult {
        for entry in self.entries {
            if entry.is_empty() {
                continue;
            }
            let asset_init_w: I80F48 = I80F48::from(entry.asset_weight_init);
            let asset_maint_w: I80F48 = I80F48::from(entry.asset_weight_maint);

            check!(
                asset_init_w >= I80F48::ZERO && asset_init_w <= I80F48::ONE,
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
            .entries
            .iter()
            .filter(|e| !e.is_empty())
            .map(|e| e.collateral_bank_emode_tag)
            .collect();

        if non_empty_tags.windows(2).any(|w| w[0] == w[1]) {
            return err!(MarginfiError::BadEmodeConfig);
        } else {
            Ok(())
        }
    }

    pub fn find_with_tag(&self, tag: u16) -> Option<&EmodeEntry> {
        self.entries.iter().find(|e| e.tag_equals(tag))
    }
    /// True if any entries are present in the mode configuration. Typically, this is the definition
    /// of flag `EMODE_ON`
    pub fn has_entries(&self) -> bool {
        self.entries.iter().any(|e| !e.is_empty())
    }
    /// True if an emode configuration has been set (EMODE_ON)
    pub fn is_enabled(&self) -> bool {
        self.flags & EMODE_ON != 0
    }
    pub fn set_emode_enabled(&mut self, enabled: bool) {
        if enabled {
            self.flags |= EMODE_ON;
        } else {
            self.flags &= !EMODE_ON;
        }
    }
}

pub const APPLIES_TO_ISOLATED: u16 = 1;

assert_struct_size!(EmodeEntry, 40);
assert_struct_align!(EmodeEntry, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout, Debug,
)]
pub struct EmodeEntry {
    /// emode_tag of the bank(s) whose collateral you wish to treat preferrentially.
    pub collateral_bank_emode_tag: u16,
    /// * APPLIES_TO_ISOLATED (1) - if set, isolated banks with this tag also benefit. If not set,
    ///   isolated banks continue to offer zero collateral, even if they use this tag.
    /// * 2, 4, 8, 16, 32, etc - reserved for future use
    pub flags: u8,
    // To next 8-byte multiple
    pub pad0: [u8; 5],
    /// Note: If set below the collateral bank's weight, does nothing.
    pub asset_weight_init: WrappedI80F48,
    /// Note: If set below the collateral bank's weight, does nothing.
    pub asset_weight_maint: WrappedI80F48,
}

impl EmodeEntry {
    pub fn is_empty(&self) -> bool {
        self.collateral_bank_emode_tag == EMODE_TAG_EMPTY
    }
    pub fn tag_equals(&self, tag: u16) -> bool {
        self.collateral_bank_emode_tag == tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed_macro::types::I80F48;

    fn create_generic_entry(tag: u16) -> EmodeEntry {
        EmodeEntry {
            collateral_bank_emode_tag: tag,
            flags: 0,
            pad0: [0u8; 5],
            asset_weight_init: I80F48!(0.7).into(),
            asset_weight_maint: I80F48!(0.8).into(),
        }
    }

    #[test]
    fn test_emode_valid_entries() {
        let mut settings = EmodeSettings::zeroed();
        settings.entries[0] = create_generic_entry(1);
        settings.entries[1] = create_generic_entry(2);
        settings.entries[2] = create_generic_entry(3);
        // Note: The remaining entries stay zeroed (and are skipped during validation).
        assert!(settings.validate_entries().is_ok());
    }

    #[test]
    fn test_emode_invalid_duplicate_tags() {
        let mut settings = EmodeSettings::zeroed();
        settings.entries[0] = create_generic_entry(1);
        settings.entries[1] = create_generic_entry(1); // Duplicate tag: 1.
        settings.entries[2] = create_generic_entry(2);
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
        settings.entries[0] = entry;
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
        settings.entries[0] = entry;
        let result = settings.validate_entries();
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), MarginfiError::BadEmodeConfig.into());
    }
}
