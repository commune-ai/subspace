use super::*;

#[must_use]
pub fn is_copying_irrational<T: pallet_subspace::Config>(
    ConsensusSimulationResult {
        creation_block,
        max_encryption_period,
        copier_margin,
        cumulative_avg_delegate_divs,
        cumulative_copier_divs,
        ..
    }: ConsensusSimulationResult<T>,
    block_number: u64,
) -> (bool, I64F64) {
    let encryption_window_len = block_number.saturating_sub(creation_block);
    if encryption_window_len >= max_encryption_period {
        return (true, I64F64::from_num(0));
    }

    let one = I64F64::from_num(1);
    let threshold = one.saturating_add(copier_margin).saturating_mul(cumulative_avg_delegate_divs);
    let delta = cumulative_copier_divs.saturating_sub(threshold);
    (delta.is_negative(), delta)
}

pub fn calculate_avg_delegate_divs<T>(
    yuma_output: &ConsensusOutput<T>,
    copier_uid: u16,
    delegation_fee: Percent,
) -> Option<I64F64>
where
    T: pallet_subspace::Config + pallet_subnet_emission::Config,
{
    let fee_factor = I64F64::from_num(100)
        .saturating_sub(I64F64::from_num(delegation_fee.deconstruct()))
        .checked_div(I64F64::from_num(100))?;

    let (total_active_stake, total_dividends) = yuma_output
        .dividends
        .iter()
        .enumerate()
        .filter(|&(i, &div)| i as u16 != copier_uid && div != 0)
        .try_fold(
            (I64F64::from_num(0), I64F64::from_num(0)),
            |(stake_acc, div_acc), (i, &div)| {
                let stake = yuma_output
                    .modules
                    .stake_original
                    .get(i)
                    .copied()
                    .unwrap_or(I64F64::from_num(0));
                let dividend = I64F64::from_num(div);

                Some((
                    stake_acc.saturating_add(stake),
                    div_acc.saturating_add(dividend),
                ))
            },
        )?;

    dbg!(total_active_stake, total_dividends);

    if total_active_stake == I64F64::from_num(0) {
        return Some(I64F64::from_num(0));
    }

    let average_dividends = total_dividends.checked_div(total_active_stake)?;

    let copier_stake = yuma_output
        .modules
        .stake_original
        .get(copier_uid as usize)
        .copied()
        .unwrap_or(I64F64::from_num(0));

    dbg!(average_dividends.saturating_mul(fee_factor).saturating_mul(copier_stake).into())
}

pub fn get_copier_stake<T>(active_stake: u64) -> u64
where
    T: pallet_subspace::Config + pallet::Config,
{
    MeasuredStakeAmount::<T>::get().mul_floor(active_stake)
}
