use super::*;

// Copying Profitbility Math
// =========================

#[must_use]
pub fn is_copying_irrational<T: pallet_subspace::Config>(
    ConsensusSimulationResult {
        black_box_age,
        max_encryption_period,
        copier_margin,
        cumulative_avg_delegate_divs,
        cumulative_copier_divs,
        ..
    }: ConsensusSimulationResult<T>,
) -> (bool, I64F64) {
    if black_box_age >= max_encryption_period {
        return (true, I64F64::from_num(0));
    }
    let one = I64F64::from_num(1);
    let threshold = one.saturating_add(copier_margin).saturating_mul(cumulative_avg_delegate_divs);
    let delta = cumulative_copier_divs.saturating_sub(threshold);
    (delta.is_negative(), delta)
}

pub fn calculate_avg_delegate_divs<T: pallet_subspace::Config + pallet_subnet_emission::Config>(
    yuma_output: &ConsensusOutput<T>,
    copier_uid: u16,
    delegation_fee: Percent,
) -> Option<I64F64> {
    let subnet_id = yuma_output.subnet_id;
    let copier_idx = copier_uid as usize;
    let fee_factor = I64F64::from_num(100)
        .saturating_sub(I64F64::from_num(delegation_fee.deconstruct()))
        .checked_div(I64F64::from_num(100))?;

    let (total_stake, total_dividends) = yuma_output
        .dividends
        .iter()
        .enumerate()
        .filter(|&(i, &div)| i != copier_idx && div != 0)
        .try_fold(
            (I64F64::from_num(0), I64F64::from_num(0)),
            |(stake_acc, div_acc), (i, &div)| {
                // TODO:
                // This has to go, use only stake from the consensus params output, we can not be
                // acesing runtime storage here
                let stake = I64F64::from_num(get_uid_deleg_stake::<T>(i as u16, subnet_id));
                let dividend = I64F64::from_num(div);
                Some((
                    stake_acc.saturating_add(stake),
                    div_acc.saturating_add(dividend),
                ))
            },
        )?;

    let average_dividends = total_dividends.checked_div(total_stake)?;
    let copier_stake = I64F64::from_num(get_uid_deleg_stake::<T>(copier_uid, subnet_id));

    average_dividends.saturating_mul(fee_factor).saturating_mul(copier_stake).into()
}

// TODO:
// This has to go, use only stake from the consensus params output, we can not be acesing runtime
// storage here
#[inline]
fn get_uid_deleg_stake<T>(module_id: u16, subnet_id: u16) -> u64
where
    T: pallet_subspace::Config,
{
    let deleg_stake = SubspaceModule::<T>::get_key_for_uid(subnet_id, module_id)
        .map_or(0, |key| SubspaceModule::<T>::get_delegated_stake(&key));

    deleg_stake
}

// TODO:
// Same here, aces stake from consensus params output
pub fn get_copier_stake<T>(subnet_id: u16) -> u64
where
    T: pallet_subspace::Config + pallet::Config,
{
    let subnet_stake: u64 = Active::<T>::get(subnet_id)
        .iter()
        .enumerate()
        .filter(|&(_, &is_active)| is_active)
        .map(|(uid, _)| get_uid_deleg_stake::<T>(uid as u16, subnet_id))
        .sum();

    MeasuredStakeAmount::<T>::get().mul_floor(subnet_stake)
}
