//! AXON Protocol — SimpleEscrow (Soroban)
//!
//! Trustless escrow for agent-to-agent payments on Stellar.
//! Client deposits; provider delivers; client releases (or both parties cancel).
//! Protocol fee (0.3% default, capped 5%) on release funds the AXON treasury.
//!
//! Port of the Solidity SimpleEscrow. Milestone 2 for SCF #46. WIP — not audited.

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowState {
    Open,        // funded, waiting for release/cancel
    Released,    // paid out to provider
    Cancelled,   // refunded to client
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub client: Address,
    pub provider: Address,
    pub token: Address,
    pub amount: i128,
    pub fee_bps: u32,          // protocol fee in basis points
    pub state: EscrowState,
    pub created_at: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,          // fee treasury/admin
    DefaultFeeBps,
    Escrow(u64),
    NextId,
}

const FEE_CAP_BPS: u32 = 500; // 5%

#[contract]
pub struct SimpleEscrow;

#[contractimpl]
impl SimpleEscrow {
    pub fn init(env: Env, admin: Address, default_fee_bps: u32) {
        if env.storage().persistent().get::<DataKey, Address>(&DataKey::Admin).is_some() {
            panic!("already initialized");
        }
        if default_fee_bps > FEE_CAP_BPS { panic!("fee too high"); }
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::DefaultFeeBps, &default_fee_bps);
        env.storage().persistent().set(&DataKey::NextId, &1u64);
    }

    /// Client opens an escrow: deposit `amount` of `token` held by this contract.
    pub fn open(env: Env, client: Address, provider: Address, token: Address, amount: i128) -> u64 {
        client.require_auth();
        if amount <= 0 { panic!("bad amount"); }
        if client == provider { panic!("same party"); }

        token::TokenClient::new(&env, &token)
            .transfer(&client, &env.current_contract_address(), &amount);

        let id: u64 = env.storage().persistent().get(&DataKey::NextId).unwrap_or(1);
        let fee_bps: u32 = env.storage().persistent().get(&DataKey::DefaultFeeBps).unwrap_or(30);
        let esc = Escrow {
            client,
            provider,
            token,
            amount,
            fee_bps,
            state: EscrowState::Open,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::Escrow(id), &esc);
        env.storage().persistent().set(&DataKey::NextId, &(id + 1));
        id
    }

    /// Client releases funds to provider (minus protocol fee).
    pub fn release(env: Env, escrow_id: u64) {
        let esc = Self::_get(&env, escrow_id);
        esc.client.require_auth();
        if esc.state != EscrowState::Open { panic!("escrow not open"); }

        let fee = esc.amount * esc.fee_bps as i128 / 10_000;
        let payout = esc.amount - fee;
        let tok = token::TokenClient::new(&env, &esc.token);
        tok.transfer(&env.current_contract_address(), &esc.provider, &payout);
        if fee > 0 {
            let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
            tok.transfer(&env.current_contract_address(), &admin, &fee);
        }
        Self::_set_state(&env, escrow_id, esc, EscrowState::Released);
    }

    /// Mutual cancel: both client and provider must approve; refund to client.
    pub fn mutual_cancel(env: Env, escrow_id: u64) {
        let esc = Self::_get(&env, escrow_id);
        esc.client.require_auth();
        esc.provider.require_auth();
        if esc.state != EscrowState::Open { panic!("escrow not open"); }

        token::TokenClient::new(&env, &esc.token)
            .transfer(&env.current_contract_address(), &esc.client, &esc.amount);
        Self::_set_state(&env, escrow_id, esc, EscrowState::Cancelled);
    }

    // ---------- Views ----------
    pub fn get_escrow(env: Env, escrow_id: u64) -> Escrow {
        Self::_get(&env, escrow_id)
    }

    fn _get(env: &Env, escrow_id: u64) -> Escrow {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .unwrap_or_else(|| panic!("escrow not found"))
    }

    fn _set_state(env: &Env, escrow_id: u64, mut esc: Escrow, state: EscrowState) {
        esc.state = state;
        env.storage().persistent().set(&DataKey::Escrow(escrow_id), &esc);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};
    use soroban_sdk::token::StellarAssetClient as SAC;

    #[test]
    fn open_release_with_fee() {
        let env = Env::default();
        let contract_id = env.register(SimpleEscrow, ());
        let client = SimpleEscrowClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);  // client
        let bob = Address::generate(&env);    // provider (agent)

        env.mock_all_auths();
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let tok = sac.address();
        SAC::new(&env, &tok).mint(&alice, &10_000);

        client.init(&admin, &30); // 0.3% fee

        // open escrow 5000
        let id = client.open(&alice, &bob, &tok, &5_000);
        let esc = client.get_escrow(&id);
        assert_eq!(esc.state, EscrowState::Open);
        assert_eq!(esc.amount, 5_000);

        // release: provider gets 5000 - 15 (0.3%) = 4985; treasury gets 15
        client.release(&id);
        let esc = client.get_escrow(&id);
        assert_eq!(esc.state, EscrowState::Released);
        assert_eq!(TokenClient::new(&env, &tok).balance(&bob), 4_985);
        assert_eq!(TokenClient::new(&env, &tok).balance(&admin), 15);
    }

    #[test]
    fn mutual_cancel_refunds() {
        let env = Env::default();
        let contract_id = env.register(SimpleEscrow, ());
        let client = SimpleEscrowClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let bob = Address::generate(&env);

        env.mock_all_auths();
        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let tok = sac.address();
        SAC::new(&env, &tok).mint(&alice, &10_000);
        client.init(&admin, &30);

        let id = client.open(&alice, &bob, &tok, &2_000);
        client.mutual_cancel(&id);
        let esc = client.get_escrow(&id);
        assert_eq!(esc.state, EscrowState::Cancelled);
        assert_eq!(TokenClient::new(&env, &tok).balance(&alice), 10_000);
    }
}
