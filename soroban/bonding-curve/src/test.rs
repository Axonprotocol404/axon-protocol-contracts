#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String, Vec,
};

fn create_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone()).address()
}

// Stub registry mirroring the interface used by the contract
#[contract]
pub struct StubRegistry;

#[contractimpl]
impl StubRegistry {
    pub fn init(env: Env, admin: Address) { admin.require_auth(); }
    pub fn register(env: Env, owner: Address, _operator: Address, _meta: String) -> u64 {
        owner.require_auth();
        let n: u64 = env.storage().persistent().get(&CKey::N).unwrap_or(0) + 1;
        env.storage().persistent().set(&CKey::N, &n);
        env.storage().persistent().set(&CKey::Owner(n), &owner);
        n
    }
    pub fn agents_of(env: Env, owner: Address) -> Vec<u64> {
        let n: u64 = env.storage().persistent().get(&CKey::N).unwrap_or(0);
        let mut v = Vec::new(&env);
        for i in 1..=n {
            if let Some(o) = env.storage().persistent().get::<CKey, Address>(&CKey::Owner(i)) {
                if o == owner { v.push_back(i); }
            }
        }
        v
    }
    pub fn get_agent(env: Env, agent_id: u64) -> RAgent {
        RAgent {
            id: agent_id,
            owner: Address::generate(&env),
            operator: Address::generate(&env),
            metadata_uri: String::from_str(&env, "x"),
            active: true,
            reputation: 0,
            task_count: 0,
            registered_at: 0,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CKey { N, Owner(u64) }

fn setup(env: &Env) -> (Address, Address, Address, Address, Address, Address, Address) {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let admin = Address::generate(env);
    let dev = Address::generate(env);
    let buyer = Address::generate(env);
    let fee_recipient = Address::generate(env);
    let token_admin = Address::generate(env);

    let registry_id = env.register(StubRegistry, ());
    let reg = RegClientX::new(env, &registry_id);
    reg.init(&admin);
    reg.register(&dev, &dev, &String::from_str(env, "meta"));

    let token = create_token(env, &token_admin);
    let native = create_token(env, &token_admin);

    let curve_id_addr = env.register(BondingCurve, ());
    let bc = crate::BondingCurveClient::new(env, &curve_id_addr);
    bc.init(&admin, &registry_id, &native, &150, &fee_recipient); // 1.5%

    let sac = StellarAssetClient::new(env, &token);
    sac.mint(&dev, &1_000_000_0000000000); // plenty
    let nsac = StellarAssetClient::new(env, &native);
    nsac.mint(&buyer, &1_000_000_0000000000);

    (admin, dev, buyer, fee_recipient, token, native, curve_id_addr)
}

#[test]
fn test_curve_full_flow() {
    let env = Env::default();
    let (_admin, dev, buyer, fee_recipient, token, native, curve_addr) = setup(&env);
    let bc = crate::BondingCurveClient::new(&env, &curve_addr);
    let token_client = TokenClient::new(&env, &token);
    let native_client = TokenClient::new(&env, &native);

    // 1M tokens, base 1000 stroops (0.0001 XLM), slope 1000 per 1M tokens
    let amount = 1_000_000_0000000i128; // 1M tokens (7 decimals)
    let base = 1000i128;
    let slope = 1000i128;
    let ends = env.ledger().timestamp() + 86_400;

    let cid = bc.create_curve(&dev, &token, &amount, &base, &slope, &ends);
    assert_eq!(cid, 0);
    assert_eq!(token_client.balance(&curve_addr), amount);

    // price at supply 0 = 1000 stroops per whole token
    assert_eq!(bc.current_price(&cid), 1000);

    // Buy 1,000,000 tokens (1M units, i.e. 1 whole token in token-units of 1e7? — treat token_amount in whole-token micro-units)
    // Buy in 1M-token buckets: buy 1_000_000 (units of 1 token each at 7dp scale → but token has 7 decimals).
    // To keep math simple: token_amount counts whole tokens (1e7 stroops each in token terms).
    let buy_amount = 1_000_000i128;
    // cost = price_at(0)*1M = 1000 * 1_000_000 = 1e9 stroops = 100 XLM
    let cost = bc.quote_buy(&cid, &buy_amount);
    assert_eq!(cost, 1_000_000_000i128);
    bc.buy(&buyer, &cid, &buy_amount, &cost); // max_cost = exact
    assert_eq!(bc.get_holding(&cid, &buyer), buy_amount);

    // Price now at 2000 stroops (sold = 1M)
    assert_eq!(bc.current_price(&cid), 2000);

    // Sell half back: refund = integral [0.5M, 1M) = price(0.5M)*0.5M = 2000*500_000 = 1e9? No:
    // buckets: [0.5M,1M) all within bucket 0..1M → price_at(500_000)=base+0*slope=1000.
    // cost_between(500_000, 1_000_000) = 1000 * 500_000 = 5e8 stroops. fee 1.5% = 7.5e6, net 492.5e6
    let sell_amount = 500_000i128;
    let refund_gross = 500_000_000i128;
    bc.sell(&buyer, &cid, &sell_amount, &(refund_gross * 985 / 1000));
    assert_eq!(bc.get_holding(&cid, &buyer), 500_000);

    // Finalize after end: contract XLM balance = 1e9 (buy) - 492.5e6 (net refund) = 507.5e6
    env.ledger().with_mut(|l| l.timestamp = ends + 1);
    bc.finalize(&cid);
    let bal_after = native_client.balance(&curve_addr);
    assert_eq!(bal_after, 0);
    // fee = 1.5% of 507.5e6 = 7.6125e6 → recipient balance
    // Contract balance at finalize = 1e9 (buy) - 492.5e6 (net refund) - 7.5e6 (sell fee) = 500e6
    // fee collected twice: 1.5% of sell refund (7.5e6) + 1.5% of remaining balance at finalize (7.5e6)
    let sell_fee = 500_000_000i128 * 150 / 10_000;      // 7_500_000
    let fin_fee = 500_000_000i128 * 150 / 10_000;        // 7_500_000
    assert_eq!(native_client.balance(&fee_recipient), sell_fee + fin_fee);
    // dev got the rest + unsold tokens (500_000 tokens)
    let dev_xlm = 500_000_000i128 - fin_fee;
    assert_eq!(native_client.balance(&dev), dev_xlm);
}

#[test]
#[should_panic(expected = "dev not registered")]
fn test_unregistered_dev_rejected() {
    let env = Env::default();
    let (_a, dev, _b, _f, token, _n, curve_addr) = setup(&env);
    // dev registered in setup; register a stranger instead
    let stranger = Address::generate(&env);
    let bc = crate::BondingCurveClient::new(&env, &curve_addr);
    bc.create_curve(&stranger, &token, &1_000_000, &1000, &1000, &(env.ledger().timestamp() + 100));
}
