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
pub struct Proposal<AccountId, Balance> {
	pub proposer: AccountId,
	pub votes: u128,
	pub details: BoundedVec<u8, ConstU32<128>>,
	pub org_id: u64,
	pub recipe_id: u128,
	pub statue: ProposalStatue,
	pub payment_requested: Balance,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Org<Balance> {
	pub name: BoundedVec<u8, ConstU32<128>>,
	pub member_count: u64,
	pub total_shares: Balance,
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
	use crate::{Action, Member, Org, Proposal, ProposalStatue, Recipe, Triger};
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use primitives::Balance;
	use sp_runtime::traits::{AccountIdConversion, One};
	use sp_std::vec::Vec;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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
	pub(super) type MapOrg<T: Config> = StorageMap<_, Twox64Concat, u64, Org<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u64, Twox64Concat, T::AccountId, Member<BalanceOf<T>>>;
	#[pallet::storage]
	#[pallet::getter(fn member_orgs)]
	pub type MemberOrgs<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, u64, ()>;

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
		Proposal<T::AccountId, BalanceOf<T>>,
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
		Summoned(T::AccountId, Vec<u8>, BalanceOf<T>),
		Donated(T::AccountId, u64, BalanceOf<T>),
		ProposalSubmited(T::AccountId, u64, BalanceOf<T>, Vec<u8>),
		VoteSubmited(T::AccountId, u64, u64),
		Transfering(u64, u64, BalanceOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,

		OrgIdMustExist,
		AccountMustInOrg,
		ProposalIdMustExist,
		ProposalMustInProcessing,
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
		pub fn summon(origin: OriginFor<T>, name: Vec<u8>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let next_org_id = NextOrgId::<T>::get().unwrap_or_default();

			let _ = T::Currency::transfer(
				&who,
				&Self::org_account(next_org_id),
				amount,
				ExistenceRequirement::KeepAlive,
			);

			let org = Org {
				name: name.clone().try_into().unwrap(),
				member_count: 1,
				total_shares: amount,
			};
			MapOrg::<T>::insert(next_org_id, org);

			let member = Member { shares: amount };
			Members::<T>::insert(next_org_id, &who, member);
			MemberOrgs::<T>::insert(&who, next_org_id, ());

			NextOrgId::<T>::put(next_org_id.saturating_add(One::one()));

			Self::deposit_event(Event::Summoned(who, name, amount));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn donate(origin: OriginFor<T>, org_id: u64, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(MapOrg::<T>::contains_key(org_id), Error::<T>::OrgIdMustExist);

			let _ = T::Currency::transfer(
				&who,
				&Self::org_account(org_id),
				amount,
				ExistenceRequirement::KeepAlive,
			);

			let org = MapOrg::<T>::get(org_id).unwrap();

			let member_count_add = match Members::<T>::get(org_id, &who) {
				Some(_) => 0,
				None => 1,
			};

			MapOrg::<T>::insert(
				org_id,
				Org {
					member_count: org.member_count + member_count_add,
					total_shares: org.total_shares + amount,
					..org
				},
			);

			let member = match Members::<T>::get(org_id, &who) {
				Some(v) => Member { shares: v.shares + amount, ..v },
				None => Member { shares: amount },
			};

			Members::<T>::insert(org_id, &who, member);
			MemberOrgs::<T>::insert(&who, org_id, ());

			Self::deposit_event(Event::Donated(who, org_id, amount));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			org_id: u64,
			payment_requested: BalanceOf<T>,
			detail: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(MapOrg::<T>::contains_key(org_id), Error::<T>::OrgIdMustExist);

			let next_proposal_id = NextProposalId::<T>::get(org_id).unwrap_or_default();

			let proposal = Proposal {
				proposer: who.clone(),
				votes: Default::default(),
				details: detail.clone().try_into().unwrap(),
				org_id,
				recipe_id: Default::default(),
				statue: ProposalStatue::Processing,
				payment_requested,
			};

			Proposals::<T>::insert(org_id, next_proposal_id, proposal);
			NextProposalId::<T>::insert(org_id, next_proposal_id.saturating_add(One::one()));

			Self::deposit_event(Event::ProposalSubmited(who, org_id, payment_requested, detail));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_vote(origin: OriginFor<T>, org_id: u64, proposal_id: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(MapOrg::<T>::contains_key(org_id), Error::<T>::OrgIdMustExist);
			ensure!(
				Proposals::<T>::contains_key(org_id, proposal_id),
				Error::<T>::ProposalIdMustExist
			);
			ensure!(Members::<T>::get(org_id, &who).is_some(), Error::<T>::AccountMustInOrg);

			let proposal = Proposals::<T>::get(org_id, proposal_id).unwrap();
			// proposal must in Processing
			ensure!(
				ProposalStatue::Processing == proposal.statue,
				Error::<T>::ProposalMustInProcessing
			);

			let org = MapOrg::<T>::get(org_id).unwrap();

			let is_pass = (proposal.votes + 1) > (org.member_count / 3 * 2).into();

			if is_pass {
				Proposals::<T>::insert(
					org_id,
					proposal_id,
					Proposal {
						votes: proposal.votes + 1,
						statue: ProposalStatue::Transfering,
						..proposal
					},
				);

				Self::deposit_event(Event::Transfering(
					org_id,
					proposal_id,
					proposal.payment_requested,
				));
			} else {
				Proposals::<T>::insert(
					org_id,
					proposal_id,
					Proposal { votes: proposal.votes + 1, ..proposal },
				);
			}

			//todo check votes > (org.member_count * 2 / 3), then create transfer
			// triger/action/recipe

			Self::deposit_event(Event::VoteSubmited(who, org_id, proposal_id));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn stop_proposal_transfer(
			origin: OriginFor<T>,
			_org_id: u64,
			_proposal_index: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			//todo do in version 2

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_recipe_done_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			_recipe_id: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			//todo yivei

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
			//todo yivei

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
			//todo yivei

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn org_account(org_id: u64) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(org_id)
		}
	}
}
