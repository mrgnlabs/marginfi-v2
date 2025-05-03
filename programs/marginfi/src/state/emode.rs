use std::collections::BTreeMap;

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
    /// preferential weights. When a new stablecoin is added that is considered riskier, it may get
    /// a new, less favorable emode tag, and eventually get upgraded to the same one as the other
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

    pub emode_config: EmodeConfig,
}

assert_struct_size!(EmodeConfig, 400);
assert_struct_align!(EmodeConfig, 8);
#[repr(C)]
#[derive(
    AnchorDeserialize, AnchorSerialize, Copy, Clone, Zeroable, Pod, PartialEq, Eq, TypeLayout, Debug,
)]
/// An emode configuration. Each bank has one such configuration, but this may also be the
/// intersection of many configurations (see `reconcile_emode_configs`). For example, the risk
/// engine creates such an intersection from all the emode config of all banks the user is borrowing
/// from.
pub struct EmodeConfig {
    pub entries: [EmodeEntry; MAX_EMODE_ENTRIES],
}

impl EmodeConfig {
    /// Creates an EmodeConfig from a slice of EmodeEntry items.
    /// Entries will be sorted by tag.
    /// Panics if more than MAX_EMODE_ENTRIES are provided.
    /// * No heap allocation
    pub fn from_entries(entries: &[EmodeEntry]) -> Self {
        let count = entries.len();
        if count > MAX_EMODE_ENTRIES {
            panic!(
                "Too many EmodeEntry items {:?}, maximum allowed {:?}",
                count, MAX_EMODE_ENTRIES
            );
        }

        let mut config = Self::zeroed();
        for (i, entry) in entries.iter().enumerate() {
            config.entries[i] = *entry;
        }
        config.entries[..count].sort_by_key(|e| e.collateral_bank_emode_tag);

        config
    }

    pub fn find_with_tag(&self, tag: u16) -> Option<&EmodeEntry> {
        if tag == EMODE_TAG_EMPTY {
            return None;
        }
        self.entries.iter().find(|e| e.tag_equals(tag))
    }
    /// True if any entries are present in the mode configuration. Typically, this is the definition
    /// of flag `EMODE_ON`
    pub fn has_entries(&self) -> bool {
        self.entries.iter().any(|e| !e.is_empty())
    }
}

impl Default for EmodeSettings {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl EmodeSettings {
    pub fn validate_entries(&self) -> MarginfiResult {
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
                MarginfiError::InvalidConfig
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
    /// emode_tag of the bank(s) whose collateral you wish to treat preferentially.
    pub collateral_bank_emode_tag: u16,
    /// * APPLIES_TO_ISOLATED (1) - (NOT YET IMPLEMENTED) if set, isolated banks with this tag
    ///   also benefit. If not set, isolated banks continue to offer zero collateral, even if they
    ///   use this tag.
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

/// Users who borrow multiple e-mode assets at the same time get the LEAST FAVORABLE treatment
/// between the borrowed assets, regardless of the amount of each asset borrowed. For example, if
/// borrowing an LST and USDC against SOL, the user would normally get an emode benefit for LST/SOL,
/// but since they are also borrowing USDC, they get only standard rates.
///
/// Returns the INTERSECTION of EmodeConfigs. If passed configs have the same
/// `collateral_bank_emode_tag`, we will take the one that has:
/// 1) The lesser of the flags (e.g. if just one has APPLIES_TO_ISOLATED, we take flags = 0)
/// 2) The lesser of both asset_weight_init and asset_weight_maint
///
/// If one config has a collateral_bank_emode_tag and the others do not, ***we don't make an
/// EmodeEntry for it at all***, i.e. there is no benefit for that collateral
///
/// * Note: Takes a generic iterator as input to avoid heap allocating a Vec.
///
/// ***Example 1***
/// * bank | tag | flags | init | maint
/// * 0       101    1       70     75
/// * 1       101    0       60     80
/// - Result
/// * tag | flags | init | maint
/// * 101    0       60     75
///
///
/// ***Example 2***
/// * bank  | tag | flags | init | maint
/// * 0       99     1       70     75
/// * 1       101    0       60     80
/// - Result
/// * tag | flags | init | maint
/// * empty
///
///
/// ***Example 3***
/// * bank  | tag | flags | init | maint
/// * 0       101    1       70     75
/// * 1       101    0       60     80
/// * 2       101    0       60     80 (note this bank has multiple entries)
/// * 2       99     0       60     80
/// - Result
/// * tag | flags | init | maint
/// * 101    0       60     75
pub fn reconcile_emode_configs<I>(configs: I) -> EmodeConfig
where
    I: IntoIterator<Item = EmodeConfig>,
{
    // TODO benchmark this in the mock program
    let mut iter = configs.into_iter();
    // Pull off the first config (if any)
    let first = match iter.next() {
        None => return EmodeConfig::zeroed(),
        Some(cfg) => cfg,
    };

    let mut merged_entries: BTreeMap<u16, (EmodeEntry, usize)> = BTreeMap::new();
    let mut num_configs = 1;

    // A helper to merge an EmodeConfig into the map
    let mut merge_cfg = |cfg: EmodeConfig| {
        for entry in cfg.entries.iter() {
            if entry.is_empty() {
                continue;
            }
            let tag = entry.collateral_bank_emode_tag;
            merged_entries
                .entry(tag)
                .and_modify(|(merged, cnt)| {
                    merged.flags = merged.flags.min(entry.flags);
                    let cur_i: I80F48 = merged.asset_weight_init.into();
                    let new_i: I80F48 = entry.asset_weight_init.into();
                    if new_i < cur_i {
                        merged.asset_weight_init = entry.asset_weight_init;
                    }
                    let cur_m: I80F48 = merged.asset_weight_maint.into();
                    let new_m: I80F48 = entry.asset_weight_maint.into();
                    if new_m < cur_m {
                        merged.asset_weight_maint = entry.asset_weight_maint;
                    }
                    *cnt += 1;
                })
                .or_insert((*entry, 1));
        }
    };

    // First config
    merge_cfg(first);

    // All following configs
    for cfg in iter {
        num_configs += 1;
        merge_cfg(cfg);
    }

    // Cllect only those tags seen in *every* config:
    let mut buf: [EmodeEntry; MAX_EMODE_ENTRIES] = [EmodeEntry::zeroed(); MAX_EMODE_ENTRIES];
    let mut buf_len = 0;

    for (_tag, (entry, cnt)) in merged_entries {
        // if cnt of appearances = num of configs, then it was in every config.
        if cnt == num_configs {
            buf[buf_len] = entry;
            buf_len += 1;
        }
    }

    // Sort what we have and pad the rest with zeroed space
    EmodeConfig::from_entries(&buf[..buf_len])
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed_macro::types::I80F48;

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
