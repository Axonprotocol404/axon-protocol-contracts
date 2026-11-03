//! AXON Protocol — AgentWallet (Soroban)
//!
//! Cap-guarded wallet for an AI agent on Stellar.
//! Owner (human) sets hard on-chain limits; operator (agent key) may only
//! send XLM/native to whitelisted addresses within per-tx and daily limits.
//! Owner can freeze, rotate operator, and withdraw freely.
//!
//! Port of the Solidity AgentWallet. Milestone 1 for SCF #46. WIP — not audited.

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Map};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Limits {
    pub per_tx: i128,   // stroops per transaction
    pub daily: i128,    // stroops per 24h window
}

#[contracttype]
pub enum DataKey {
    Owner,
    Operator,
    Limits,
    Whitelist,          // Map<Address, bool>
    Frozen,
    SpentInWindow,      // i128 spent in current 24h window
    WindowStart,        // u64 ledger timestamp of current window
    Registry,           // Option: AXON AgentRegistry contract address (gate)
    AgentId,            // registry agent id this wallet belongs to
    Token,              // SAC (Stellar Asset Contract) address, e.g. native XLM SAC
}

#[contract]
pub struct AgentWallet;

#[contractimpl]
impl AgentWallet {
    /// Owner initializes the wallet with operator + limits.
    pub fn init(env: Env, owner: Address, operator: Address, token: Address, per_tx: i128, daily: i128) {
        if env.storage().persistent().get::<DataKey, Address>(&DataKey::Owner).is_some() {
            panic!("already initialized");
        }
        env.storage().persistent().set(&DataKey::Owner, &owner);
        env.storage().persistent().set(&DataKey::Operator, &operator);
        env.storage().persistent().set(&DataKey::Token, &token);
        env.storage().persistent().set(&DataKey::Limits, &Limits { per_tx, daily });
        env.storage().persistent().set(&DataKey::Whitelist, &Map::<Address, bool>::new(&env));
        env.storage().persistent().set(&DataKey::Frozen, &false);
        env.storage().persistent().set(&DataKey::SpentInWindow, &0i128);
        env.storage().persistent().set(&DataKey::WindowStart, &env.ledger().timestamp());
    }

    // ---------- Owner functions ----------

    fn owner(env: &Env) -> Address {
        env.storage().persistent().get(&DataKey::Owner).unwrap()
    }

    pub fn set_operator(env: Env, new_operator: Address) {
        let o = Self::owner(&env);
        o.require_auth();
        env.storage().persistent().set(&DataKey::Operator, &new_operator);
    }

    pub fn set_limits(env: Env, per_tx: i128, daily: i128) {
        let o = Self::owner(&env);
        o.require_auth();
        env.storage().persistent().set(&DataKey::Limits, &Limits { per_tx, daily });
    }

    pub fn whitelist(env: Env, addr: Address, allowed: bool) {
        let o = Self::owner(&env);
        o.require_auth();
        let mut map: Map<Address, bool> = env.storage().persistent().get(&DataKey::Whitelist).unwrap();
        map.set(addr, allowed);
        env.storage().persistent().set(&DataKey::Whitelist, &map);
    }

    pub fn set_frozen(env: Env, frozen: bool) {
        let o = Self::owner(&env);
        o.require_auth();
        env.storage().persistent().set(&DataKey::Frozen, &frozen);
    }

    /// Owner withdraws freely (bypasses all limits).
    pub fn owner_withdraw(env: Env, to: Address, amount: i128) {
        let o = Self::owner(&env);
        o.require_auth();
        if amount <= 0 { panic!("bad amount"); }
        let tok: Address = env.storage().persistent().get(&DataKey::Token).unwrap();
        token::TokenClient::new(&env, &tok).transfer(&env.current_contract_address(), &to, &amount);
    }

    /// Owner binds this wallet to an AXON AgentRegistry entry (for the pad/escrow gates).
    pub fn bind_agent(env: Env, registry: Address, agent_id: u64) {
        let o = Self::owner(&env);
        o.require_auth();
        env.storage().persistent().set(&DataKey::Registry, &registry);
        env.storage().persistent().set(&DataKey::AgentId, &agent_id);
    }

    // ---------- Operator (agent) functions ----------

    /// Agent pays `to` (must be whitelisted) within per-tx and daily caps.
    pub fn agent_pay(env: Env, to: Address, amount: i128) {
        if env.storage().persistent().get(&DataKey::Frozen).unwrap_or(true) {
            panic!("wallet frozen");
        }
        let operator: Address = env.storage().persistent().get(&DataKey::Operator).unwrap();
        operator.require_auth();

        // Whitelist check
        let map: Map<Address, bool> = env.storage().persistent().get(&DataKey::Whitelist).unwrap();
        if map.get(to.clone()).unwrap_or(false) != true {
            panic!("recipient not whitelisted");
        }

        // Per-tx cap
        let limits: Limits = env.storage().persistent().get(&DataKey::Limits).unwrap();
        if amount <= 0 || amount > limits.per_tx {
            panic!("over per-tx limit");
        }

        // Daily cap: reset window if 24h elapsed
        let now = env.ledger().timestamp();
        let mut window_start: u64 = env.storage().persistent().get(&DataKey::WindowStart).unwrap();
        let mut spent: i128 = env.storage().persistent().get(&DataKey::SpentInWindow).unwrap_or(0);
        if now >= window_start + 86_400 {
            window_start = now;
            spent = 0;
            env.storage().persistent().set(&DataKey::WindowStart, &window_start);
        }
        if spent + amount > limits.daily {
            panic!("over daily limit");
        }
        spent += amount;
        env.storage().persistent().set(&DataKey::SpentInWindow, &spent);

        let tok: Address = env.storage().persistent().get(&DataKey::Token).unwrap();
        token::TokenClient::new(&env, &tok).transfer(&env.current_contract_address(), &to, &amount);
    }

    // ---------- Views ----------

    pub fn get_owner(env: Env) -> Address {
        env.storage().persistent().get(&DataKey::Owner).unwrap()
    }

    pub fn get_limits(env: Env) -> Limits {
        env.storage().persistent().get(&DataKey::Limits).unwrap()
    }

    pub fn is_frozen(env: Env) -> bool {
        env.storage().persistent().get(&DataKey::Frozen).unwrap_or(true)
    }

    pub fn is_whitelisted(env: Env, addr: Address) -> bool {
        let map: Map<Address, bool> = env.storage().persistent().get(&DataKey::Whitelist).unwrap();
        map.get(addr).unwrap_or(false)
    }

    pub fn spent_in_window(env: Env) -> i128 {
        env.storage().persistent().get(&DataKey::SpentInWindow).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};

    const PER_TX: i128 = 1_000;
    const DAILY: i128 = 2_500;

    #[test]
    fn whitelist_and_caps_enforced() {
        let env = Env::default();
        let contract_id = env.register(AgentWallet, ());
        let client = AgentWalletClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let operator = Address::generate(&env);
        let payee = Address::generate(&env);
        let attacker = Address::generate(&env);

        env.mock_all_auths();
        let sac = env.register_stellar_asset_contract_v2(owner.clone());
        let token_addr = sac.address();
        // fund the wallet contract via admin mint
        StellarAssetClient::new(&env, &token_addr).mint(&contract_id, &100_000);
        client.init(&owner, &operator, &token_addr, &PER_TX, &DAILY);
        client.whitelist(&payee, &true);

        // 1. allowed: within per-tx
        client.agent_pay(&payee, &800);
        assert_eq!(client.spent_in_window(), 800);

        // 2. blocked: non-whitelisted
        let r = client.try_agent_pay(&attacker, &100);
        assert!(r.is_err());

        // 3. blocked: over per-tx limit
        let r = client.try_agent_pay(&payee, &(PER_TX + 1));
        assert!(r.is_err());

        // 4. allowed: brings total to 1800 (< daily 2500)
        client.agent_pay(&payee, &1000);
        assert_eq!(client.spent_in_window(), 1800);

        // 5. blocked: would exceed daily (1800 + 800 = 2600 > 2500)
        let r = client.try_agent_pay(&payee, &800);
        assert!(r.is_err());

        // 6. freeze blocks operator, owner can still withdraw
        client.set_frozen(&true);
        let r = client.try_agent_pay(&payee, &100);
        assert!(r.is_err());
        client.set_frozen(&false);
        client.owner_withdraw(&owner, &100);
    }
}
