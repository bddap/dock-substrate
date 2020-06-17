#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch::DispatchResult,
                    traits::Get, sp_runtime::{print, SaturatedConversion} };
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_std::prelude::Vec;

/// The pallet's configuration trait.
pub trait Trait: system::Trait + pallet_session::Trait {
    // Add other types and constants required to configure this pallet.

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    type MinSessionLength: Get<u32>;

    type MaxActiveValidators: Get<u8>;
}

// This pallet's storage items.
decl_storage! {
	// It is important to update your storage name so that your pallet's
	// storage items are isolated from other pallets.
	trait Store for Module<T: Trait> as PoAModule {
		CurrentValidators get(fn current_validators) config(): Vec<T::AccountId>;

		// TODO: Remove the count
		CurrentValidatorCount get(fn current_validator_count) config(): u8;

		//NextSessionChangeAt get(fn next_session_change_at) config(): u32;
		NextSessionChangeAt get(fn next_session_change_at) config(): T::BlockNumber;

		AddValidators get(fn validators_to_add): Vec<T::AccountId>;

		RemoveValidators get(fn validators_to_remove): Vec<T::AccountId>;
	}
}

// The pallet's events
decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		// New validator added.
		ValidatorAdded(AccountId),

		// Validator removed.
		ValidatorRemoved(AccountId),
	}
);

// The pallet's errors
decl_error! {
	/// Errors for the module.
	pub enum Error for Module<T: Trait> {
		NoValidators,
	}
}

// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;

		/*/// Add a new validator using root/sudo privileges.
		///
		/// New validator's session keys should be set in session module before calling this.
		pub fn add_validator(origin, validator_id: T::AccountId, do_now: bool) -> DispatchResult {
		    // TODO: Check the origin is Master
			ensure_root(origin)?;

			let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
			validators.push(validator_id.clone());
			<Validators<T>>::put(validators);
			// Calling rotate_session to queue the new session keys.
			<session::Module<T>>::rotate_session();
			Self::deposit_event(RawEvent::ValidatorAdded(validator_id));

			// Triggering rotate session again for the queued keys to take effect.
			Flag::put(true);
			Ok(())
		}

		/// Remove a validator using root/sudo privileges.
		pub fn remove_validator(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
			ensure_root(origin)?;
			let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
			// Assuming that this will be a PoA network for enterprise use-cases,
			// the validator count may not be too big; the for loop shouldn't be too heavy.
			// In case the validator count is large, we need to find another way.
			for (i, v) in validators.clone().into_iter().enumerate() {
				if v == validator_id {
					validators.swap_remove(i);
				}
			}
			<Validators<T>>::put(validators);
			// Calling rotate_session to queue the new session keys.
			<session::Module<T>>::rotate_session();
			Self::deposit_event(RawEvent::ValidatorRemoved(validator_id));

			// Triggering rotate session again for the queued keys to take effect.
			Flag::put(true);
			Ok(())
		}*/
	}
}

/// Indicates to the session module if the session should be rotated.
/// We set this flag to true when we add/remove a validator.
impl<T: Trait> pallet_session::ShouldEndSession<T::BlockNumber> for Module<T> {
    fn should_end_session(_now: T::BlockNumber) -> bool {
        print("Called should_end_session");
        print(Self::next_session_change_at().saturated_into::<u32>());
        let current_block_no = <system::Module<T>>::block_number();
        // TODO: Remove once sure the following panic is never triggered
        if current_block_no.saturated_into::<u32>() > Self::next_session_change_at().saturated_into::<u32>() {
            panic!("Current block number > next_session_change_at");
            print(current_block_no.saturated_into::<u32>())
        }
        current_block_no == Self::next_session_change_at()
    }
}

/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Trait> pallet_session::SessionManager<T::AccountId> for Module<T> {
    fn new_session(_: u32) -> Option<Vec<T::AccountId>> {
        // Flag is set to false so that the session doesn't keep rotating.
        //Flag::put(false);
        print("Called new_session");
        // Check for error on empty validator set
        let mut current_validators = Self::current_validators();
        if current_validators.len() == 0 {
            None
        } else {
            let current_block_no = <system::Module<T>>::block_number().saturated_into::<u32>();
            let min_session_len = T::MinSessionLength::get();
            let rem = min_session_len % current_validators.len() as u32;
            let session_len = if rem == 0 {
                min_session_len
            } else {
                min_session_len + current_validators.len() as u32 - rem
            };
            let next_session_at = current_block_no + session_len;
            print(next_session_at);
            <NextSessionChangeAt<T>>::put(T::BlockNumber::from(next_session_at));
            // let current_block_no = <system::Module<T>>::block_number();
            // print("next_session_change_at");
            // print(Self::next_session_change_at());
            Some(current_validators)
        }
        /*match Self::current_validators() {
            Some(mut current_validators) => {

            }
            None => None
        }*/
    }

    // SessionIndex is u32 but comes from sp_staking pallet. Since staking is not needed for now, not
    // importing the pallet.
    fn start_session(_: u32) {}
    fn end_session(_: u32) {}
}

/*/// Implementation of Convert trait for mapping ValidatorId with AccountId.
/// This is mainly used to map stash and controller keys.
/// In this module, for simplicity, we just return the same AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
    fn convert(account: T::AccountId) -> Option<T::AccountId> {
        Some(account)
    }
}*/

