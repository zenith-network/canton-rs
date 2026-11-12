use canton_proto::com::digitalasset::canton::topology::admin::v30 as proto;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ForceFlag {
    /// Required when authorizing adding a topology transaction on behalf of another node.
    AlienMember,

    #[deprecated(
        since = "Canton protocol v32",
        note = "increasing ledger time record time tolerance does not require a force flag"
    )]
    LedgerTimeRecordTimeToleranceIncrease,

    /// Required when vetting unknown packages (not uploaded).
    AllowUnknownPackage,

    /// Required when vetting a package with unvetted dependencies.
    AllowUnvettedDependencies,

    /// Required when disabling a party with active contracts.
    DisablePartyWithActiveContracts,

    /// Required when using a key that is not suitable to sign a topology transaction.
    ///
    /// Using this force flag likely causes the transaction to be rejected at a later stage of the
    /// processing.
    AllowUnvalidatedSigningKeys,

    /// Required when increasing the submission time record time tolerance.
    PreparationTimeRecordTimeToleranceIncrease,

    /// Required when we want to change all participants' permissions to observation while the party
    /// is still a signatory of a contract.
    AllowInsufficientParticipantPermissionForSignatoryParty,

    /// Required when changing the party-to-participant mapping, that would result in insufficient
    /// signatory-assigning participants and thus the assignment would be stuck.
    AllowInsufficientSignatoryAssigningParticipantsForParty,

    /// Required when vetting a package that fails upgrade checking.
    AllowVetIncompatibleUpgrades,

    /// Required when submitting dynamic synchronizer parameters that have out-of-bounds values.
    AllowOutOfBoundsValue,

    /// Required when changing the confirming threshold to a value higher than the number of
    /// confirming participants.
    AllowConfirmingThresholdCannotBeMet,
}

impl From<ForceFlag> for proto::ForceFlag {
    fn from(value: ForceFlag) -> Self {
        match value {
            ForceFlag::AlienMember => proto::ForceFlag::AlienMember,
            #[allow(deprecated)]
            ForceFlag::LedgerTimeRecordTimeToleranceIncrease => {
                proto::ForceFlag::LedgerTimeRecordTimeToleranceIncrease
            }
            ForceFlag::AllowUnknownPackage => proto::ForceFlag::AllowUnknownPackage,
            ForceFlag::AllowUnvettedDependencies => proto::ForceFlag::AllowUnvettedDependencies,
            ForceFlag::DisablePartyWithActiveContracts => {
                proto::ForceFlag::DisablePartyWithActiveContracts
            }
            ForceFlag::AllowUnvalidatedSigningKeys => proto::ForceFlag::AllowUnvalidatedSigningKeys,
            ForceFlag::PreparationTimeRecordTimeToleranceIncrease => {
                proto::ForceFlag::PreparationTimeRecordTimeToleranceIncrease
            }
            ForceFlag::AllowInsufficientParticipantPermissionForSignatoryParty => {
                proto::ForceFlag::AllowInsufficientParticipantPermissionForSignatoryParty
            }
            ForceFlag::AllowInsufficientSignatoryAssigningParticipantsForParty => {
                proto::ForceFlag::AllowInsufficientSignatoryAssigningParticipantsForParty
            }
            ForceFlag::AllowVetIncompatibleUpgrades => {
                proto::ForceFlag::AllowVetIncompatibleUpgrades
            }
            ForceFlag::AllowOutOfBoundsValue => proto::ForceFlag::AllowOutOfBoundsValue,
            ForceFlag::AllowConfirmingThresholdCannotBeMet => {
                proto::ForceFlag::AllowConfirmingThresholdCannotBeMet
            }
        }
    }
}

impl From<ForceFlag> for i32 {
    fn from(value: ForceFlag) -> Self {
        proto::ForceFlag::from(value).into()
    }
}
