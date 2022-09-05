use crate::{mock::*, Error, Member, Org, Proposal, ProposalStatue};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(DonateModule::do_something(Origin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(DonateModule::something(), Some(42));
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(DonateModule::cause_error(Origin::signed(1)), Error::<Test>::NoneValue);
	});
}

#[test]
fn test_summon() {
	new_test_ext().execute_with(|| {
		assert_ok!(DonateModule::summon(Origin::signed(1), vec![1, 2, 3], 1));

		assert_eq!(
			DonateModule::map_org(0).unwrap(),
			Org { name: vec![1, 2, 3].try_into().unwrap(), member_count: 1, total_shares: 1 }
		);

		assert_eq!(DonateModule::members(0, 1).unwrap(), Member { shares: 1 });
		assert_eq!(DonateModule::member_orgs(1, 0).is_some(), true);
	});
}

#[test]
fn test_donate() {
	new_test_ext().execute_with(|| {
		assert_noop!(DonateModule::donate(Origin::signed(1), 0, 3), Error::<Test>::OrgIdMustExist);

		assert_ok!(DonateModule::summon(Origin::signed(1), vec![1, 2, 3], 1));
		assert_ok!(DonateModule::donate(Origin::signed(2), 0, 3));

		assert_eq!(
			DonateModule::map_org(0).unwrap(),
			Org { name: vec![1, 2, 3].try_into().unwrap(), member_count: 2, total_shares: 4 }
		);
		assert_eq!(DonateModule::members(0, 1).unwrap(), Member { shares: 1 });
		assert_eq!(DonateModule::members(0, 2).unwrap(), Member { shares: 3 });
		assert_eq!(DonateModule::member_orgs(1, 0).is_some(), true);
		assert_eq!(DonateModule::member_orgs(2, 0).is_some(), true);

		assert_ok!(DonateModule::donate(Origin::signed(2), 0, 3));
		assert_eq!(
			DonateModule::map_org(0).unwrap(),
			Org { name: vec![1, 2, 3].try_into().unwrap(), member_count: 2, total_shares: 7 }
		);
		assert_eq!(DonateModule::members(0, 2).unwrap(), Member { shares: 6 });
	});
}

#[test]
fn test_submit_proposal() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			DonateModule::submit_proposal(Origin::signed(2), 0, 3, vec![1, 2, 3]),
			Error::<Test>::OrgIdMustExist
		);

		assert_ok!(DonateModule::summon(Origin::signed(1), vec![1, 2, 3], 1));
		assert_ok!(DonateModule::submit_proposal(Origin::signed(2), 0, 3, vec![1, 2, 3]));

		assert_eq!(
			DonateModule::proposals(0, 0).unwrap(),
			Proposal {
				proposer: 2,
				votes: Default::default(),
				details: vec![1, 2, 3].try_into().unwrap(),
				org_id: 0,
				recipe_id: Default::default(),
				statue: ProposalStatue::Processing,
				payment_requested: 3,
			}
		);
	});
}

#[test]
fn test_submit_vote() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			DonateModule::submit_vote(Origin::signed(2), 0, 0),
			Error::<Test>::OrgIdMustExist
		);

		assert_ok!(DonateModule::summon(Origin::signed(1), vec![1, 2, 3], 1));
		assert_noop!(
			DonateModule::submit_vote(Origin::signed(2), 0, 0),
			Error::<Test>::ProposalIdMustExist
		);

		assert_ok!(DonateModule::submit_proposal(Origin::signed(2), 0, 3, vec![1, 2, 3]));
		assert_noop!(
			DonateModule::submit_vote(Origin::signed(2), 0, 0),
			Error::<Test>::AccountMustInOrg
		);

		assert_ok!(DonateModule::donate(Origin::signed(2), 0, 3));
		assert_ok!(DonateModule::donate(Origin::signed(3), 0, 3));
		assert_ok!(DonateModule::donate(Origin::signed(4), 0, 3));
		assert_ok!(DonateModule::donate(Origin::signed(5), 0, 3));

		assert_ok!(DonateModule::submit_vote(Origin::signed(2), 0, 0));
		assert_ok!(DonateModule::submit_vote(Origin::signed(3), 0, 0));
		assert_ok!(DonateModule::submit_vote(Origin::signed(4), 0, 0));
		assert_eq!(
			DonateModule::proposals(0, 0).unwrap(),
			Proposal {
				proposer: 2,
				votes: 3,
				details: vec![1, 2, 3].try_into().unwrap(),
				org_id: 0,
				recipe_id: Default::default(),
				statue: ProposalStatue::Transfering,
				payment_requested: 3,
			}
		);

		assert_noop!(
			DonateModule::submit_vote(Origin::signed(5), 0, 0),
			Error::<Test>::ProposalMustInProcessing
		);
	});
}
