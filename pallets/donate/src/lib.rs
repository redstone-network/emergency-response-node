#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::ConstU32, BoundedVec};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::cmp::{Eq, PartialEq};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// DAO
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProposalStatue {
	//Canceled,
	Processing,
	//DidNotPass,
	Transfering, //had Passed
	//StopedTransfer,
	AllDone,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Proposal<AccountId> {
	pub proposer: AccountId,
	pub votes: u128,
	pub details: BoundedVec<u8, ConstU32<128>>,
	pub org_id: u64,
	pub recipe_id: u128,
	pub statue: ProposalStatue,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Org {
	pub name: BoundedVec<u8, ConstU32<128>>,
	pub member_count: u64,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Member<Balance> {
	pub shares: Balance, //donate balance
}

//DIFTTT
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Triger {
	Timer(u64, u64, u64), //insert_time, timer_seconds, max_times
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Action<AccountId, Balance> {
	TranferToken(AccountId, Balance),
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Recipe {
	triger_id: u64,
	action_id: u64,
	enable: bool,
	times: u64,
	max_times: u64,
	done: bool,
	last_triger_timestamp: u64,
	force_stop: bool,
}

#[frame_support::pallet]
pub mod pallet {
	use crate::{Action, Member, Org, Proposal, Recipe, Triger};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use primitives::Balance;
	use sp_std::vec::Vec;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn next_org_id)]
	pub type NextOrgId<T: Config> = StorageValue<_, u64>;
	#[pallet::storage]
	#[pallet::getter(fn map_org)]
	pub(super) type MapOrg<T: Config> = StorageMap<_, Twox64Concat, u64, Org>;
	#[pallet::storage]
	#[pallet::getter(fn map_total_shares)]
	pub(super) type MapTotalShares<T: Config> = StorageMap<_, Twox64Concat, u64, Balance>;

	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u64,
		Twox64Concat,
		T::AccountId,
		Member<Balance>,
		OptionQuery,
	>;
	#[pallet::storage]
	#[pallet::getter(fn member_orgs)]
	pub type MemberOrgs<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, u64, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_proposal_id)]
	pub type NextProposalId<T: Config> = StorageMap<_, Twox64Concat, u64, u64>;
	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u64,
		Twox64Concat,
		u64,
		Proposal<T::AccountId>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn triger_owner)]
	pub type TrigerOwner<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, u64, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn action_owner)]
	pub type ActionOwner<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, u64, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn recipe_owner)]
	pub type RecipeOwner<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, u64, (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn map_triger)]
	pub(super) type MapTriger<T: Config> = StorageMap<_, Twox64Concat, u64, Triger>;

	#[pallet::storage]
	#[pallet::getter(fn map_action)]
	pub(super) type MapAction<T: Config> =
		StorageMap<_, Twox64Concat, u64, Action<T::AccountId, Balance>>;

	#[pallet::storage]
	#[pallet::getter(fn map_recipe)]
	pub(super) type MapRecipe<T: Config> = StorageMap<_, Twox64Concat, u64, Recipe>;

	#[pallet::storage]
	#[pallet::getter(fn next_triger_id)]
	pub type NextTrigerId<T: Config> = StorageValue<_, u64>;
	#[pallet::storage]
	#[pallet::getter(fn next_action_id)]
	pub type NextActionId<T: Config> = StorageValue<_, u64>;
	#[pallet::storage]
	#[pallet::getter(fn next_recipe_id)]
	pub type NextRecipeId<T: Config> = StorageValue<_, u64>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				},
			}
		}

		#[pallet::weight(0)]
		pub fn summon(origin: OriginFor<T>, _name: Vec<u8>, _amount: Balance) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn donate(origin: OriginFor<T>, _org_id: u64, _amount: Balance) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			_org_id: u64,
			_payment_requested: Balance,
			_details: Vec<u8>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_vote(
			origin: OriginFor<T>,
			_org_id: u64,
			_proposal_index: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn stop_proposal_transfer(
			origin: OriginFor<T>,
			_org_id: u64,
			_proposal_index: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_recipe_done_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			_recipe_id: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_recipe_triger_timestamp_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			_recipe_id: u64,
			_timestamp: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_recipe_triger_times_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			_recipe_id: u64,
			_times: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
