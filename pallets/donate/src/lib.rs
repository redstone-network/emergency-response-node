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
	pub approve_votes: u128,
	pub deny_votes: u128,
	pub details: BoundedVec<u8, ConstU32<2048>>,
	pub org_id: u64,
	pub recipe_id: u128,
	pub statue: ProposalStatue,
	pub payment_requested: Balance,
	pub payment_frequency: u32,
}

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Org<AccountId, Balance> {
	pub name: BoundedVec<u8, ConstU32<128>>,
	pub member_count: u64,
	pub total_shares: Balance,
	pub treasury_id: AccountId,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Member<Balance> {
	pub shares: Balance, //donate balance
}

//DIFTTT
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Triger {
	Timer(u64, u64, u64), //insert_time, timer_m_seconds, max_times
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Action<AccountId, Balance> {
	TranferToken(u64, u64, AccountId, Balance), //org_id, proposal_id, reciver, amout
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
		traits::{Currency, ExistenceRequirement, UnixTime},
		PalletId,
	};
	use frame_system::{
		offchain::{CreateSignedTransaction, SubmitTransaction},
		pallet_prelude::*,
	};
	use sp_runtime::{
		offchain::{
			storage::StorageValueRef,
			storage_lock::{BlockAndTime, StorageLock},
			Duration,
		},
		traits::{AccountIdConversion, BlockNumberProvider, One},
	};
	use sp_std::{collections::btree_map::BTreeMap, prelude::*, str, vec::Vec};

	const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds
	const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
	const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		type TimeProvider: UnixTime;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
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
	pub(super) type MapOrg<T: Config> =
		StorageMap<_, Twox64Concat, u64, Org<T::AccountId, BalanceOf<T>>>;

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
		StorageMap<_, Twox64Concat, u64, Action<T::AccountId, BalanceOf<T>>>;

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
		TransferFrequency(u64, u64, u64, T::AccountId, T::AccountId, BalanceOf<T>), /* org_id,
		                                                                             * proposal_id,
		                                                                             * times, from,
		                                                                             * to, amout */

		RecipeDone(u64),
		RecipeTrigerTimesUpdated(u64, u64, u64),
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

		OffchainUnsignedTxError,
		RecipeIdNotExist,
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
				treasury_id: Self::org_account(next_org_id),
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
				approve_votes: Default::default(),
				deny_votes: Default::default(),
				details: detail.clone().try_into().unwrap(),
				org_id,
				recipe_id: Default::default(),
				statue: ProposalStatue::Processing,
				payment_requested,
				payment_frequency: 12u32,
			};

			Proposals::<T>::insert(org_id, next_proposal_id, proposal);
			NextProposalId::<T>::insert(org_id, next_proposal_id.saturating_add(One::one()));

			Self::deposit_event(Event::ProposalSubmited(who, org_id, payment_requested, detail));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_vote(
			origin: OriginFor<T>,
			org_id: u64,
			proposal_id: u64,
			vote_unit: u8,
		) -> DispatchResult {
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
						approve_votes: proposal.approve_votes + if vote_unit == 0 { 1 } else { 0 },
						deny_votes: proposal.deny_votes + if vote_unit == 1 { 1 } else { 0 },
						statue: ProposalStatue::Transfering,
						..proposal.clone()
					},
				);

				let proposer = proposal.proposer;

				let frequency = 12u64;
				//triger
				let triger_id = NextTrigerId::<T>::get().unwrap_or_default();
				let triger = Triger::Timer(T::TimeProvider::now().as_secs(), 60, frequency);
				MapTriger::<T>::insert(triger_id, triger.clone());
				TrigerOwner::<T>::insert(&proposer, triger_id, ());
				NextTrigerId::<T>::put(triger_id.saturating_add(One::one()));

				//action
				let action_id = NextActionId::<T>::get().unwrap_or_default();
				let action = Action::TranferToken(
					org_id,
					proposal_id,
					proposer.clone(),
					proposal.payment_requested,
				);
				MapAction::<T>::insert(action_id, action.clone());
				ActionOwner::<T>::insert(&proposer, action_id, ());
				NextActionId::<T>::put(action_id.saturating_add(One::one()));

				let recipe_id = NextRecipeId::<T>::get().unwrap_or_default();
				let recipe = Recipe {
					triger_id,
					action_id,
					enable: true,
					times: 0,
					max_times: frequency,
					done: false,
					last_triger_timestamp: 0,
					force_stop: false,
				};
				MapRecipe::<T>::insert(recipe_id, recipe.clone());
				RecipeOwner::<T>::insert(&proposer, recipe_id, ());
				NextRecipeId::<T>::put(recipe_id.saturating_add(One::one()));

				Self::deposit_event(Event::Transfering(
					org_id,
					proposal_id,
					proposal.payment_requested,
				));
			} else {
				Proposals::<T>::insert(
					org_id,
					proposal_id,
					Proposal {
						votes: proposal.votes + 1,
						approve_votes: proposal.approve_votes + if vote_unit == 0 { 1 } else { 0 },
						deny_votes: proposal.deny_votes + if vote_unit == 1 { 1 } else { 0 },
						..proposal
					},
				);
			}

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
			recipe_id: u64,
		) -> DispatchResult {
			ensure_none(origin)?;

			ensure!(MapRecipe::<T>::contains_key(&recipe_id), Error::<T>::RecipeIdNotExist);

			MapRecipe::<T>::try_mutate(recipe_id, |recipe| -> DispatchResult {
				if let Some(recipe) = recipe {
					recipe.done = true;
					Self::deposit_event(Event::RecipeDone(recipe_id));
				}
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_recipe_triger_times_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			recipe_id: u64,
			times: u64,
			timestamp: u64,
		) -> DispatchResult {
			ensure_none(origin)?;

			ensure!(MapRecipe::<T>::contains_key(&recipe_id), Error::<T>::RecipeIdNotExist);

			MapRecipe::<T>::try_mutate(recipe_id, |recipe| -> DispatchResult {
				if let Some(recipe) = recipe {
					recipe.times = times;
					recipe.last_triger_timestamp = timestamp;
					Self::deposit_event(Event::RecipeTrigerTimesUpdated(
						recipe_id, times, timestamp,
					));
				}
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn tranfer_unsigned(
			origin: OriginFor<T>,
			org_id: u64,
			proposal_id: u64,
			times: u64,
			_block_number: T::BlockNumber,
			reciver: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;

			log::info!("###### tranfer_unsigned. org_id {:?}", org_id);

			T::Currency::transfer(
				&Self::org_account(org_id),
				&reciver,
				amount,
				ExistenceRequirement::KeepAlive,
			)
			.map_err(|e| {
				log::error!("###tranfer_unsigned Failed in T::Currency::transfer {:?}", e);
				<Error<T>>::OffchainUnsignedTxError
			});

			Self::deposit_event(Event::TransferFrequency(
				org_id,
				proposal_id,
				times,
				Self::org_account(org_id),
				reciver,
				amount,
			));

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("###### Hello from pallet-difttt-offchain-worker.");

			// let parent_hash = <frame_system::Pallet<T>>::block_hash(block_number - 1u32.into());
			// log::info!("###### Current block: {:?} (parent hash: {:?})", block_number,
			// parent_hash);

			let timestamp_now = T::TimeProvider::now();
			log::info!("###### Current time: {:?} ", timestamp_now.as_secs());

			let store_hashmap_recipe = StorageValueRef::persistent(b"difttt_ocw::recipe_task");

			let mut map_recipe_task: BTreeMap<u64, Recipe>;
			if let Ok(Some(info)) = store_hashmap_recipe.get::<BTreeMap<u64, Recipe>>() {
				map_recipe_task = info;
			} else {
				map_recipe_task = BTreeMap::new();
			}

			let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
				b"offchain-demo::lock",
				LOCK_BLOCK_EXPIRATION,
				Duration::from_millis(LOCK_TIMEOUT_EXPIRATION),
			);

			let mut map_running_action_recipe_task: BTreeMap<u64, Recipe> = BTreeMap::new();
			if let Ok(_guard) = lock.try_lock() {
				for (recipe_id, recipe) in MapRecipe::<T>::iter() {
					if recipe.enable && !recipe.done {
						if !map_recipe_task.contains_key(&recipe_id) {
							log::info!("###### map_recipe_task.insert {:?}", &recipe_id);
							map_recipe_task.insert(
								recipe_id,
								Recipe {
									triger_id: recipe.triger_id,
									action_id: recipe.action_id,
									enable: true,
									times: 0,
									max_times: 0,
									done: false,
									last_triger_timestamp: 0,
									force_stop: false,
								},
							);
						}
					} else {
						log::info!("###### map_recipe_task.remove {:?}", &recipe_id);
						map_recipe_task.remove(&recipe_id);
					};
				}

				for (recipe_id, recipe) in map_recipe_task.iter_mut() {
					let triger = MapTriger::<T>::get(recipe.triger_id);

					match triger {
						Some(Triger::Timer(insert_time, timer_seconds, _max_times)) =>
							if insert_time + recipe.times * timer_seconds < timestamp_now.as_secs()
							{
								(*recipe).times += 1;
								log::info!(
									"###### recipe {:?} Current Triger times: {:?} ",
									recipe_id,
									recipe.times
								);

								map_running_action_recipe_task.insert(*recipe_id, recipe.clone());
							},

						_ => {},
					}
				}

				store_hashmap_recipe.set(&map_recipe_task);
			};

			//todo run action
			for (recipe_id, recipe) in map_running_action_recipe_task.iter() {
				let action = MapAction::<T>::get(recipe.action_id);
				match action {
					Some(Action::TranferToken(org_id, proposal_id, reciver, amount)) => {
						match Self::offchain_unsigned_tranfer(
							block_number,
							org_id,
							proposal_id,
							recipe.times,
							reciver,
							amount,
						) {
							Ok(_) => {
								log::info!("###### submit_unsigned_transaction ok");
								log::info!("###### TranferToken ok");

								match Self::offchain_unsigned_tx_update_recipe_triger_times(
									block_number,
									*recipe_id,
									recipe.times,
									timestamp_now.as_secs(),
								) {
									Ok(_) => {
										log::info!("###### submit_unsigned_transaction ok");
										log::info!(
											"###### offchain_unsigned_tx_update_recipe_triger_time ok"
										);
									},
									Err(e) => {
										log::info!(
											"###### submit_unsigned_transaction error  {:?}",
											e
										);
									},
								};

								if recipe.times >= recipe.max_times {
									match Self::offchain_unsigned_tx_recipe_done(
										block_number,
										*recipe_id,
									) {
										Ok(_) => {
											log::info!("###### submit_unsigned_transaction ok");
											log::info!(
												"###### offchain_unsigned_tx_recipe_done ok"
											);
										},
										Err(e) => {
											log::info!(
												"###### submit_unsigned_transaction error  {:?}",
												e
											);
										},
									};
								}
							},
							Err(e) => {
								log::info!("###### submit_unsigned_transaction error  {:?}", e);
							},
						};

						// match Self::offchain_unsigned_tx_update_recipe_triger_times(
						// 	block_number,
						// 	*recipe_id,
						// 	recipe.times,
						// 	timestamp_now.as_secs(),
						// ) {
						// 	Ok(_) => {
						// 		log::info!("###### submit_unsigned_transaction ok");
						// 		log::info!(
						// 			"###### offchain_unsigned_tx_update_recipe_triger_time ok"
						// 		);
						// 	},
						// 	Err(e) => {
						// 		log::info!(
						// 			"###### submit_unsigned_transaction error  {:?}",
						// 			e
						// 		);
						// 	},
						// };
					},
					_ => {},
				}
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			let valid_tx = |provide| {
				ValidTransaction::with_tag_prefix("ocw-difttt")
					.priority(T::UnsignedPriority::get())
					.and_provides([&provide])
					.longevity(3)
					.propagate(true)
					.build()
			};

			match call {
				Call::tranfer_unsigned {
					org_id: _,
					proposal_id: _,
					times: _,
					block_number: _,
					reciver: _,
					amount: _,
				} => valid_tx(b"tranfer_unsigned".to_vec()),
				Call::update_recipe_triger_times_unsigned {
					block_number: _,
					recipe_id: _,
					times: _,
					timestamp: _,
				} => valid_tx(b"update_recipe_triger_times_unsigned".to_vec()),
				Call::set_recipe_done_unsigned { block_number: _, recipe_id: _ } =>
					valid_tx(b"set_recipe_done_unsigned".to_vec()),

				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn org_account(org_id: u64) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(org_id)
		}

		fn offchain_unsigned_tx_recipe_done(
			block_number: T::BlockNumber,
			recipe_id: u64,
		) -> Result<(), Error<T>> {
			let call = Call::set_recipe_done_unsigned { block_number, recipe_id };

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(|e| {
				log::error!(
					"###offchain_unsigned_tx_recipe_done Failed in offchain_unsigned_tx {:?}",
					e
				);
				<Error<T>>::OffchainUnsignedTxError
			})
		}

		fn offchain_unsigned_tx_update_recipe_triger_times(
			block_number: T::BlockNumber,
			recipe_id: u64,
			times: u64,
			timestamp: u64,
		) -> Result<(), Error<T>> {
			let call = Call::update_recipe_triger_times_unsigned {
				block_number,
				recipe_id,
				times,
				timestamp,
			};

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(|e| {
				log::error!("###offchain_unsigned_tx_update_recipe_triger_times Failed in offchain_unsigned_tx {:?}", e);
				<Error<T>>::OffchainUnsignedTxError
			})
		}

		fn offchain_unsigned_tranfer(
			block_number: T::BlockNumber,
			org_id: u64,
			proposal_id: u64,
			times: u64,
			reciver: T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			let call = Call::tranfer_unsigned {
				block_number,
				org_id,
				proposal_id,
				times,
				reciver,
				amount,
			};

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(|e| {
				log::error!("###offchain_unsigned_tranfer in offchain_unsigned_tx {:?}", e);
				<Error<T>>::OffchainUnsignedTxError
			})
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}
