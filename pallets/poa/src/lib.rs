#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
/// Pallet to add and remove validators.
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure,
    sp_runtime::{print, SaturatedConversion},
    traits::{Currency, Get, Imbalance, OnUnbalanced},
};
use frame_system::{self as system, ensure_root};
use log::{debug, warn};
use sp_std::prelude::Vec;

extern crate alloc;
use alloc::collections::BTreeSet;

// #[cfg(test)]
// mod tests;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QueuePosition {
    Front,
    Back,
}

/// The pallet's configuration trait.
pub trait Trait: system::Trait + pallet_session::Trait + pallet_authorship::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Epoch length in number of slots
    type MinEpochLength: Get<u64>;

    /// Maximum no. of active validators allowed
    type MaxActiveValidators: Get<u8>;

    type Currency: Currency<Self::AccountId>;
}

// This pallet's storage items.
decl_storage! {

    trait Store for Module<T: Trait> as PoAModule {
        /// List of active validators. Maximum allowed are `MaxActiveValidators`
        ActiveValidators get(fn active_validators) config(): BTreeSet<T::AccountId>;

        /// Next epoch will begin after this slot number, i.e. this slot number will be the last
        /// slot of the current epoch
        EpochEndsAt get(fn epoch_ends_at): u64;

        /// Queue of validators to become part of the active validators. Validators can be added either
        /// to the back of the queue or front by passing a flag in the add method.
        /// On epoch end or prematurely if forced, validators from the queue are taken out in FIFO order
        /// and added to the active validators unless the number of active validators reaches the max allowed.
        /// UPDATE
        QueuedValidators get(fn validator_queue): Vec<T::AccountId>;

        /// List to hold validators to remove either on the end of epoch or immediately. If a candidate validator
        /// is queued for becoming an active validator but also present in the removal list, it will
        /// not become active as this list takes precedence of queued validators. It is emptied on each
        /// epoch end
        /// UPDATE
        RemoveValidators get(fn validators_to_remove): BTreeSet<T::AccountId>;

        /// Set when a hot swap is to be performed, replacing validator id as 1st element of tuple
        /// with the 2nd one.
        HotSwap get(fn hot_swap): Option<(T::AccountId, T::AccountId)>;

        /// Transaction fees
        TxnFees get(fn txn_fees): <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

        /// Current epoch
        Epoch get(fn epoch): u32;

        /// For each epoch, validator count, starting slot, ending slot
        Epochs get(fn get_epoch_detail): map hasher(identity) u32 => (u8, u64, Option<u64>);

        /// Block produced by each validator per epoch
        EpochBlockCounts get(fn get_block_count_for_validator):
            double_map hasher(identity) u32, hasher(blake2_128_concat) T::AccountId => u64;
    }
}

// The pallet's events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        // New validator added to queue.
        ValidatorQueued(AccountId, QueuePosition),

        // Validator removed.
        ValidatorRemoved(AccountId),

        EpochBegins(u64),
    }
);

// The pallet's errors
decl_error! {
    /// Errors for the module.
    pub enum Error for Module<T: Trait> {
        MaxValidators,
        AlreadyActiveValidator,
        AlreadyQueuedForRemoval,
        NeedAtLeast1Validator,
        SwapOutFailed,
        SwapInFailed,
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Do maximum of the work in functions to `add_validator` or `remove_validator` and minimize the work in
        // `should_end_session` since that it called more frequently.

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Add a new validator to active validator set unless already a validator and the total number
        /// of validators don't exceed the max allowed count. The validator is considered for adding at
        /// the end of this epoch unless `short_circuit` is true. If a validator is already added to the queue
        /// an error will be thrown unless `short_circuit` is true, in which case it swallows the error.
        /// UPDATE
        // Weight can be 0 as its called by Master. TODO: Use signed extension to make it free
        #[weight = 0]
        pub fn enqueue_validator(origin, validator_id: T::AccountId, position: QueuePosition) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::enqueue_validator_(validator_id, position)
        }

        /// Remove the given validator from active validator set and the queued validators at the end
        /// of epoch unless `short_circuit` is true. If validator is already queued for removal, an error
        /// will be thrown unless `short_circuit` is true, in which case it swallows the error.
        /// It will not remove the validator if the removal will cause the active validator set to
        /// be empty even after considering the queued validators.
        /// UPDATE
        // Weight can be 0 as its called by Master. TODO: Use signed extension to make it free
        #[weight = 0]
        pub fn schedule_removal(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::schedule_removal_(validator_id)
        }

        /// Replace an active validator (`old_validator_id`) with a new validator (`new_validator_id`)
        /// without waiting for epoch to end. Throws error if `old_validator_id` is not active or
        /// `new_validator_id` is already active. Also useful when a validator wants to rotate his account.
        // Weight can be 0 as its called by Master. TODO: Use signed extension to make it free
        #[weight = 0]
        pub fn swap_validator(origin, old_validator_id: T::AccountId, new_validator_id: T::AccountId) -> dispatch::DispatchResult {
            ensure_root(origin)?;
            Self::swap_validator_(old_validator_id, new_validator_id)
        }

        /// Awards the complete txn fees to the block author if any and increment block count for
        /// current epoch and who authored it.
        fn on_finalize() {
            print("Finalized block");

            // Get the current block author
            let author = <pallet_authorship::Module<T>>::author();

            Self::award_txn_fees_if_any(&author);

            Self::increment_current_epoch_block_count(author)
        }
    }
}

impl<T: Trait> Module<T> {
    fn enqueue_validator_(
        validator_id: T::AccountId,
        position: QueuePosition,
    ) -> dispatch::DispatchResult {
        ensure!(
            !Self::active_validators().contains(&validator_id),
            Error::<T>::AlreadyActiveValidator
        );

        <QueuedValidators<T>>::mutate(|queue| {
            Self::remove_validator_id(&validator_id, queue);
            match position {
                QueuePosition::Front => queue.insert(0, validator_id.clone()),
                QueuePosition::Back => queue.push(validator_id.clone()),
            }
        });
        Self::deposit_event(RawEvent::ValidatorQueued(validator_id, position));

        Ok(())
    }

    fn schedule_removal_(validator_id: T::AccountId) -> dispatch::DispatchResult {
        let mut removals = Self::validators_to_remove();
        ensure!(
            removals.insert(validator_id.clone()),
            Error::<T>::AlreadyQueuedForRemoval
        );
        // potential = active ∪ queued
        let potential_validators: BTreeSet<T::AccountId> = Self::active_validators()
            .into_iter()
            .chain(Self::validator_queue())
            .collect();
        ensure!(
            !removals.is_superset(&potential_validators),
            Error::<T>::NeedAtLeast1Validator
        );
        <RemoveValidators<T>>::put(removals);
        Self::deposit_event(RawEvent::ValidatorRemoved(validator_id));
        Ok(())
    }

    fn swap_validator_(
        old_validator_id: T::AccountId,
        new_validator_id: T::AccountId,
    ) -> dispatch::DispatchResult {
        let active_validators = Self::active_validators();

        ensure!(
            !active_validators.contains(&new_validator_id),
            Error::<T>::SwapInFailed
        );
        ensure!(
            active_validators.contains(&old_validator_id),
            Error::<T>::SwapOutFailed
        );

        <HotSwap<T>>::put((old_validator_id, new_validator_id));
        Ok(())
    }

    /// Takes a validator id and a mutable vector of validator ids and remove any occurrence from
    /// the mutable vector. Returns number of removed occurrences
    fn remove_validator_id(id: &T::AccountId, validators: &mut Vec<T::AccountId>) -> usize {
        let old_size = validators.len();
        validators.retain(|v| v != id);
        old_size - validators.len()
    }

    /// Update active validator set if needed and return if the active validator set changed and the
    /// count of the new active validators.
    /// UPDATE
    fn update_active_validators_if_needed() {
        let mut active = Self::active_validators();
        let mut queue = Self::validator_queue();
        let max_validators = T::MaxActiveValidators::get() as usize;
        let removals = <RemoveValidators<T>>::take();

        debug_assert!(!queue.iter().any(|v| removals.contains(&v)));
        debug_assert!(active.is_superset(&removals));

        for v in removals {
            active.remove(&v);
        }

        debug_assert!(queue.iter().all(|q| active.contains(&q)));

        while (active.len() < max_validators) && (!queue.is_empty()) {
            active.insert(queue.remove(0));
        }

        <QueuedValidators<T>>::put(queue);
        <ActiveValidators<T>>::put(active);
    }

    /// Set next epoch duration such that it is >= `MinEpochLength` and also a multiple of the
    /// number of active validators
    fn set_current_epoch_end(current_slot_no: u64, active_validator_count: u8) -> u64 {
        let min_epoch_len = T::MinEpochLength::get();
        let active_validator_count = active_validator_count as u64;
        let rem = min_epoch_len % active_validator_count;
        let epoch_len = if rem == 0 {
            min_epoch_len
        } else {
            min_epoch_len + active_validator_count - rem
        };
        // Current slot no is part of epoch
        let epoch_ends_at = current_slot_no + epoch_len - 1;
        EpochEndsAt::put(epoch_ends_at);
        epoch_ends_at
    }

    /// Swap out from active validators if `swap` is not None and will return count of active validators
    /// in an Option
    fn swap_if_needed(swap: Option<(T::AccountId, T::AccountId)>) -> Option<u8> {
        swap.map(|(old_validator, new_validator)| Self::swap(old_validator, new_validator))
    }

    /// Swap a validator account from active validators. Swap out `old_validator_id` for `new_validator_id`.
    /// Expects the active validator set to contain `old_validator_id`. This is ensured by the extrinsic.
    /// UPDATE
    fn swap(old_validator_id: T::AccountId, new_validator_id: T::AccountId) -> u8 {
        let mut active_validators = Self::active_validators();
        debug_assert_ne!(old_validator_id, new_validator_id);
        debug_assert!(active_validators.contains(&old_validator_id));
        debug_assert!(!active_validators.contains(&new_validator_id));
        active_validators.remove(&old_validator_id);
        active_validators.insert(new_validator_id);
        <ActiveValidators<T>>::put(&active_validators);
        active_validators.len() as u8
    }

    /// Return the current slot no if accessible
    fn current_slot_no() -> Option<u64> {
        let digest = <system::Module<T>>::digest();
        let log0 = digest.logs().first()?;
        let pre_run = log0.as_pre_runtime()?;
        let s = u64::decode(&mut &pre_run.1[..]).unwrap();
        Some(s)
        // TODO: prove these assumptions:
        // Assumes that the first log is for PreRuntime digest
        // Assumes that the 2nd element of tuple is for slot no.
    }

    /// Prematurely end current epoch but keep slots multiple of no of validators
    /// Updates storage for end of current epoch
    fn short_circuit_current_epoch() -> u64 {
        let current_slot_no = Self::current_slot_no().unwrap();
        // Moving the logic to separate method to keep it testable
        Self::update_current_epoch_end_on_short_circuit(current_slot_no)
    }

    /// Updates storage for end of current epoch on premature ending of epoch.
    /// Takes the current slot no.
    fn update_current_epoch_end_on_short_circuit(current_slot_no: u64) -> u64 {
        let current_epoch_no = Self::epoch();
        let (active_validator_count, starting_slot, _) = Self::get_epoch_detail(current_epoch_no);
        let active_validator_count = active_validator_count as u64;
        let current_progress = current_slot_no - starting_slot + 1;
        let rem = current_progress % active_validator_count;
        let epoch_ends_at = if rem == 0 {
            current_slot_no
        } else {
            current_slot_no + active_validator_count - rem
        };
        EpochEndsAt::put(epoch_ends_at);
        debug!(
            target: "runtime",
            "Epoch {} prematurely ended at slot {}",
            current_epoch_no, epoch_ends_at
        );
        epoch_ends_at
    }

    /// If there is any transaction fees, credit it to the given author
    fn award_txn_fees_if_any(block_author: &T::AccountId) -> Option<u64> {
        // ------------- DEBUG START -------------
        let current_block_no = <system::Module<T>>::block_number();
        debug!(
            target: "runtime",
            "block author in finalize for {:?} is {:?}",
            current_block_no, block_author
        );

        let total_issuance = T::Currency::total_issuance().saturated_into::<u64>();
        let ab = T::Currency::free_balance(block_author).saturated_into::<u64>();
        debug!(
            target: "runtime",
            "block author's balance is {} and total issuance is {}",
            ab, total_issuance
        );
        // ------------- DEBUG END -------------

        let txn_fees = <TxnFees<T>>::take();
        let fees_as_u64 = txn_fees.saturated_into::<u64>();
        if fees_as_u64 > 0 {
            print("Depositing fees");
            // `deposit_creating` will do the issuance of tokens burnt during transaction fees
            T::Currency::deposit_creating(block_author, txn_fees);
        }

        // ------------- DEBUG START -------------
        let total_issuance = T::Currency::total_issuance().saturated_into::<u64>();
        let ab = T::Currency::free_balance(block_author).saturated_into::<u64>();
        debug!(
            target: "runtime",
            "block author's balance is {} and total issuance is {}",
            ab, total_issuance
        );
        // ------------- DEBUG END -------------

        if fees_as_u64 > 0 {
            Some(fees_as_u64)
        } else {
            None
        }
    }

    fn increment_current_epoch_block_count(block_author: T::AccountId) {
        let current_epoch_no = Self::epoch();
        let block_count = Self::get_block_count_for_validator(current_epoch_no, &block_author);
        // Not doing saturating add as its practically impossible to produce 2^64 blocks
        <EpochBlockCounts<T>>::insert(current_epoch_no, block_author, block_count + 1);
    }

    fn update_details_on_epoch_change(
        current_epoch_no: u32,
        current_slot_no: u64,
        active_validator_count: u8,
    ) {
        if current_epoch_no == 1 {
            // First epoch, no no previous epoch to update
        } else {
            // Track end of previous epoch
            let prev_epoch = current_epoch_no - 1;
            let (v, start, _) = Epochs::get(&prev_epoch);
            if v == 0 {
                // This get should never fail. But if it does, let it panic
                warn!(
                    target: "runtime",
                    "Data for previous epoch not found: {}",
                    prev_epoch
                );
                panic!();
            }
            debug!(
                target: "runtime",
                "Epoch {} ends at slot {}",
                prev_epoch, current_slot_no - 1
            );
            Epochs::insert(prev_epoch, (v, start, Some(current_slot_no - 1)))
        }

        debug!(
            target: "runtime",
            "Epoch {} begins at slot {}",
            current_epoch_no, current_slot_no
        );
        Epoch::put(current_epoch_no);
        Epochs::insert(
            current_epoch_no,
            (active_validator_count, current_slot_no, None as Option<u64>),
        );
    }
}

/// Indicates to the session module if the session should be rotated.
impl<T: Trait> pallet_session::ShouldEndSession<T::BlockNumber> for Module<T> {
    fn should_end_session(_now: T::BlockNumber) -> bool {
        let current_slot_no = Self::current_slot_no().expect(
            "current slot number is inaccessable, can't compute whether the poa session has ended",
        );
        (current_slot_no > Self::epoch_ends_at()) || <HotSwap<T>>::get().is_some()
    }
}

/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Trait> pallet_session::SessionManager<T::AccountId> for Module<T> {
    fn new_session(session_idx: u32) -> Option<Vec<T::AccountId>> {
        todo!()
        // let validators = Self::active_validators();
        // // Check for error on empty validator set. On returning None, it loads validator set from genesis
        // if validators.len() == 0 {
        //     None
        // } else {
        //     // This slot number should always be available here. If its not then panic.
        //     let current_slot_no = Self::current_slot_no().unwrap();

        //     let active_validator_count = validators.len() as u8;
        //     let current_epoch_no = session_idx - 1;

        //     Self::update_details_on_epoch_change(
        //         current_epoch_no,
        //         current_slot_no,
        //         active_validator_count,
        //     );

        //     Some(validators.into_iter().collect())
        // }
    }

    fn end_session(_: u32) {}
    fn start_session(_: u32) {}
}

/// Negative imbalance used to transfer transaction fess to block author
type NegativeImbalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

/// Transfer complete transaction fees (including tip) to the block author
impl<T: Trait> OnUnbalanced<NegativeImbalanceOf<T>> for Module<T> {
    /// There is only 1 way to have an imbalance in the system right now which is txn fees
    /// This function will store txn fees for the block in storage which is "taken out" of storage
    /// in `on_finalize`. Not retrieving block author here as that is unreliable and gives different
    /// author than the block's.
    fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
        // TODO: Remove the next 3 debug lines
        let current_fees = amount.peek();

        // ------------- DEBUG START -------------
        let total_issuance = T::Currency::total_issuance().saturated_into::<u64>();

        debug!(
            target: "runtime",
            "Current txn fees is {} and total issuance is {}",
            current_fees.saturated_into::<u64>(), total_issuance
        );

        let current_block_no = <system::Module<T>>::block_number();

        // Get the current block author
        let author = <pallet_authorship::Module<T>>::author();
        debug!(
            target: "runtime",
            "block author for {:?} is {:?}",
            current_block_no, author
        );

        // ------------- DEBUG END -------------

        <TxnFees<T>>::put(current_fees);
    }
}