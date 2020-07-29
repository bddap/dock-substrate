#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_support::sp_runtime::traits::Saturating;
use sp_std::prelude::*;
use system::RawOrigin;

const SEED: u32 = 0;
const MAX_USER_INDEX: u32 = 1000;

benchmarks! {
        _ {
            // Migrator
            let u in 1 .. MAX_USER_INDEX => ();
            // No of migrations
            let n in 1 .. 100 => ();
        }

        // Assumes recipient account does not exist which is most likely to be the case
        migrate {
            let u in ...;
            let n in ...;

            // Fuel migrator
            let existential_deposit: BalanceOf<T> = 500.into();
            let migrator: T::AccountId = account("caller", u, SEED);
            let balance = existential_deposit.saturating_mul(120.into());
		    let _ = T::Currency::make_free_balance_be(&migrator, balance);

            // Setup recipients
            let mut recipients = BTreeMap::new();
            for i in 0..n {
                // Same seed is fine as index is different
                let recipient: T::AccountId = account("recipient", i, SEED);
                recipients.insert(recipient, existential_deposit);
            }
        }: _(RawOrigin::Signed(migrator), recipients)
}
