multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_errors::{ERROR_BAD_PAYMENTS, ERROR_EMPTY_PAYMENTS};
use common_structs::{PaymentAttributesPair, PaymentsVec};
use multiversx_sc::api::BlockchainApi;
use multiversx_sc::contract_base::BlockchainWrapper;

pub struct ClaimRewardsContext<M, T>
where
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
{
    pub additional_payments: PaymentsVec<M>,
    pub additional_token_attributes: ManagedVec<M, T>,
    pub first_farm_token: PaymentAttributesPair<M, T>,
}

impl<M, T> ClaimRewardsContext<M, T>
where
    M: ManagedTypeApi + BlockchainApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + ManagedVecItem,
{
    pub fn new(
        mut payments: PaymentsVec<M>,
        farm_token_id: &TokenIdentifier<M>,
        api_wrapper: BlockchainWrapper<M>,
    ) -> Self {
        if payments.is_empty() {
            M::error_api_impl().signal_error(ERROR_EMPTY_PAYMENTS);
        }

        for p in &payments {
            if &p.token_identifier != farm_token_id {
                M::error_api_impl().signal_error(ERROR_BAD_PAYMENTS);
            }
        }

        let own_sc_address = api_wrapper.get_sc_address();
        let first_payment = payments.get(0);
        payments.remove(0);

        let first_token_data = api_wrapper.get_esdt_token_data(
            &own_sc_address,
            farm_token_id,
            first_payment.token_nonce,
        );
        let first_token_attributes: T = first_token_data.decode_attributes();

        let mut additional_token_attributes = ManagedVec::new();
        for payment in &payments {
            let token_data = api_wrapper.get_esdt_token_data(
                &own_sc_address,
                farm_token_id,
                payment.token_nonce,
            );
            let token_attributes: T = token_data.decode_attributes();
            additional_token_attributes.push(token_attributes);
        }

        ClaimRewardsContext {
            additional_payments: payments,
            additional_token_attributes,
            first_farm_token: PaymentAttributesPair {
                payment: first_payment,
                attributes: first_token_attributes,
            },
        }
    }
}

pub type CompoundRewardsContext<M, T> = ClaimRewardsContext<M, T>;
