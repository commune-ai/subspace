use super::*;

#[derive(Clone, Debug, PartialEq, Encode, Decode)]

pub struct ConsensusSimulationResult<T: pallet_subspace::Config> {
    pub cumulative_copier_divs: I64F64,
    pub cumulative_avg_delegate_divs: I64F64,
    pub copier_margin: I64F64,
    pub black_box_age: u64,
    pub max_encryption_period: u64,
    pub _phantom: PhantomData<T>,
}

impl<T: pallet_subspace::Config> Default for ConsensusSimulationResult<T> {
    fn default() -> Self {
        ConsensusSimulationResult {
            cumulative_copier_divs: I64F64::from_num(0),
            cumulative_avg_delegate_divs: I64F64::from_num(0),
            copier_margin: I64F64::from_num(0),
            black_box_age: 0,
            max_encryption_period: 0,
            _phantom: PhantomData,
        }
    }
}

impl<T: pallet_subspace::Config + pallet_subnet_emission::Config> ConsensusSimulationResult<T> {
    pub fn update(
        &mut self,
        yuma_output: ConsensusOutput<T>,
        tempo: u16,
        copier_uid: u16,
        delegation_fee: Percent,
    ) {
        let avg_delegate_divs =
            calculate_avg_delegate_divs::<T>(&yuma_output, copier_uid, delegation_fee)
                .unwrap_or_else(|| FixedI128::from(0));

        let copier_divs = yuma_output
            .dividends
            .get(copier_uid as usize)
            .map(|&div| I64F64::from_num(div))
            .unwrap_or_else(|| I64F64::from_num(0));

        self.cumulative_copier_divs = self.cumulative_copier_divs.saturating_add(copier_divs);
        self.cumulative_avg_delegate_divs =
            self.cumulative_avg_delegate_divs.saturating_add(avg_delegate_divs);
        self.black_box_age = self.black_box_age.saturating_add(u64::from(tempo));

        self.max_encryption_period = MaxEncryptionPeriod::<T>::get(yuma_output.subnet_id);
        self.copier_margin = CopierMargin::<T>::get(yuma_output.subnet_id);
    }
}

pub struct ShouldDecryptResult<T: pallet_subspace::Config> {
    pub should_decrypt: bool,
    pub simulation_result: ConsensusSimulationResult<T>,
    pub delta: I64F64,
}
