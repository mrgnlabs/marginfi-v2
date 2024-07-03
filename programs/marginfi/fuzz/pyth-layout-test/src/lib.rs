#[cfg(test)]
mod tests {
    use marginfi::state::price::pyth_sdk_solana::{
        self as new_pyth_sdk_solana, state::SolanaPriceAccount as PriceAccountV_0_10_1,
    };
    use pyth_sdk_solana::state::PriceAccount as PriceAccountV0_9_0;
    use solana_program::pubkey::Pubkey;

    #[test]
    #[rustfmt::skip]
    fn check_layout() {
        let new = PriceAccountV_0_10_1 {
            magic:          1,
            ver:            2,
            atype:          3,
            size:           4,
            ptype:          new_pyth_sdk_solana::state::PriceType::Price,
            expo:           5,
            num:            6,
            num_qt:         7,
            last_slot:      8,
            valid_slot:     9,
            ema_price:      new_pyth_sdk_solana::state::Rational {
                val:   1,
                numer: 2,
                denom: 3,
            },
            ema_conf:       new_pyth_sdk_solana::state::Rational {
                val:   1,
                numer: 2,
                denom: 3,
            },
            timestamp:      12,
            min_pub:        13,
            drv2:           14,
            drv3:           15,
            drv4:           16,
            prod:           Pubkey::new_from_array([1; 32]),
            next:           Pubkey::new_from_array([2; 32]),
            prev_slot:      19,
            prev_price:     20,
            prev_conf:      21,
            prev_timestamp: 22,
            agg:            new_pyth_sdk_solana::state::PriceInfo {
                price:    1,
                conf:     2,
                status:   new_pyth_sdk_solana::state::PriceStatus::Trading,
                corp_act: new_pyth_sdk_solana::state::CorpAction::NoCorpAct,
                pub_slot: 5,
            },
            comp:           [Default::default(); 32],
            extended:       (),
        };

        let old = PriceAccountV0_9_0 {
            magic:          1,
            ver:            2,
            atype:          3,
            size:           4,
            ptype:          pyth_sdk_solana::state::PriceType::Price,
            expo:           5,
            num:            6,
            num_qt:         7,
            last_slot:      8,
            valid_slot:     9,
            ema_price:      pyth_sdk_solana::state::Rational {
                val:   1,
                numer: 2,
                denom: 3,
            },
            ema_conf:       pyth_sdk_solana::state::Rational {
                val:   1,
                numer: 2,
                denom: 3,
            },
            timestamp:      12,
            min_pub:        13,
            drv2:           14,
            drv3:           15,
            drv4:           16,
            prod:           Pubkey::new_from_array([1; 32]),
            next:           Pubkey::new_from_array([2; 32]),
            prev_slot:      19,
            prev_price:     20,
            prev_conf:      21,
            prev_timestamp: 22,
            agg:            pyth_sdk_solana::state::PriceInfo {
                price:    1,
                conf:     2,
                status:    pyth_sdk_solana::state::PriceStatus::Trading,
                corp_act:  pyth_sdk_solana::state::CorpAction::NoCorpAct,
                pub_slot: 5,
            },
            comp:           [Default::default(); 32],
        };

        // Equal Sized?
        assert_eq!(
            std::mem::size_of::<PriceAccountV0_9_0>(),
            std::mem::size_of::<PriceAccountV_0_10_1>(),
        );

        // Equal Byte Representation?
        unsafe {
            let old_b = std::slice::from_raw_parts(
                &old as *const PriceAccountV0_9_0 as *const u8,
                std::mem::size_of::<PriceAccountV0_9_0>(),
            );
            let new_b = std::slice::from_raw_parts(
                &new as *const PriceAccountV_0_10_1 as *const u8,
                std::mem::size_of::<PriceAccountV_0_10_1>(),
            );
            assert_eq!(old_b, new_b);
        }

        // Equal Fields?
        macro_rules! check_field {
            ($field:ident) => {
                assert_eq!(*$field, new.$field, "field {} not equal", stringify!($field));
            };
            ($field:ident, $new:expr) => {
                assert_eq!(*$field, $new.$field, "field {} not equal", stringify!($field));
            };
        }

        let PriceAccountV0_9_0 {
            magic,
            ver,
            atype,
            size,
            ptype,
            expo,
            num,
            num_qt,
            last_slot,
            valid_slot,
            ema_price,
            ema_conf,
            timestamp,
            min_pub,
            drv2,
            drv3,
            drv4,
            prod,
            next,
            prev_slot,
            prev_price,
            prev_conf,
            prev_timestamp,
            agg,
            comp,
        } = bytemuck::from_bytes(bytemuck::bytes_of(&new));
        check_field!(magic);
        check_field!(ver);
        check_field!(atype);
        check_field!(size);
        check_field!(expo);
        check_field!(num);
        check_field!(num_qt);
        check_field!(last_slot);
        check_field!(valid_slot);
        check_field!(timestamp);
        check_field!(min_pub);
        check_field!(drv2);
        check_field!(drv3);
        check_field!(drv4);
        check_field!(prod);
        check_field!(next);
        check_field!(prev_slot);
        check_field!(prev_price);
        check_field!(prev_conf);
        check_field!(prev_timestamp);

        // The following require special handling
        // check_field!(ptype);
        // check_field!(ema_price);
        // check_field!(ema_conf);
        // check_field!(agg);
        // check_field!(comp);

        assert_eq!(*ptype as i32, new.ptype as u8 as i32);

        // ema
        let pyth_sdk_solana::state::Rational {
            val,
            numer,
            denom,
        } = ema_price;
        {
            check_field!(val, new.ema_price);
            check_field!(numer, new.ema_price);
            check_field!(denom, new.ema_price);
        }

        // ema_conf
        let pyth_sdk_solana::state::Rational {
            val,
            numer,
            denom,
        } = ema_conf;
        {
            check_field!(val, new.ema_conf);
            check_field!(numer, new.ema_conf);
            check_field!(denom, new.ema_conf);
        }

         // agg
         let pyth_sdk_solana::state::PriceInfo {
            price,
            conf,
            status,
            corp_act,
            pub_slot,
        } = agg;
        {
            check_field!(price, new.agg);
            check_field!(conf, new.agg);
            check_field!(pub_slot, new.agg);

            assert_eq!(*status as i32, new.agg.status as u8 as i32);
            assert_eq!(*corp_act as i32, new.agg.corp_act as u8 as i32);
        }

        // comp
        for (c, new_c) in comp.iter().zip(new.comp) {
            let pyth_sdk_solana::state::PriceComp {
                publisher,
                agg,
                latest,
            } = c;
            check_field!(publisher, new_c);
            
            // agg
            let pyth_sdk_solana::state::PriceInfo {
                price,
                conf,
                status,
                corp_act,
                pub_slot,
            } = agg;
            {
                check_field!(price, new_c.agg);
                check_field!(conf, new_c.agg);
                check_field!(pub_slot, new_c.agg);

                assert_eq!(*status as i32, new_c.agg.status as u8 as i32);
                assert_eq!(*corp_act as i32, new_c.agg.corp_act as u8 as i32);
            }

            // agg
            let pyth_sdk_solana::state::PriceInfo {
                price,
                conf,
                status,
                corp_act,
                pub_slot,
            } = latest;
            {
                check_field!(price, new_c.latest);
                check_field!(conf, new_c.latest);
                check_field!(pub_slot, new_c.latest);

                assert_eq!(*status as i32, new_c.latest.status as u8 as i32);
                assert_eq!(*corp_act as i32, new_c.latest.corp_act as u8 as i32);
            }
        }
        
    }
}
