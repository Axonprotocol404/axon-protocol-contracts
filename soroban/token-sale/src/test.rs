#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, Register},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String, Vec,
};

fn create_token(env: &Env, admin: &Address) -> Address {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    sac.address()
}

#[test]
fn test_full_sale_flow() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let admin = Address::generate(&env);
    let dev = Address::generate(&env);
    let buyer1 = Address::generate(&env);
    let buyer2 = Address::generate(&env);
    let fee_recipient = Address::generate(&env);

    // Setup registry
    let registry_id = env.register(StubRegistry, ());
    let registry_client = RegClientX::new(&env, &registry_id);
    // init registry with admin
    registry_client.init(&admin);
    // register dev as agent #1
    registry_client.register(&dev, &dev, &String::from_str(&env, "https://dev.meta"));

    // Setup token & native
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_client = TokenClient::new(&env, &token);
    let sac_client = StellarAssetClient::new(&env, &token);
    sac_client.mint(&dev, &10_000_000000000); // plenty for allocation
    sac_client.mint(&buyer1, &100_0000000);
    sac_client.mint(&buyer2, &100_0000000);

    let native = create_token(&env, &token_admin); // fake native for test
    let native_client = TokenClient::new(&env, &native);
    let native_sac = StellarAssetClient::new(&env, &native);
    native_sac.mint(&buyer1, &10_000_0000000);
    native_sac.mint(&buyer2, &10_000_0000000);

    // Setup sale contract
    let sale_id_addr = env.register(crate::TokenSale, ());
    let sale_client = TokenSaleClient::new(&env, &sale_id_addr);
    sale_client.init(&admin, &registry_id, &native, &150, &fee_recipient); // 1.5% fee

    // Dev creates sale: price = 1 XLM per 1000 tokens (price in stroops per whole token)
    // token_price = 1_0000000 stroops XLM per 1 token → 1000 tokens per XLM
    let sale_starts = env.ledger().timestamp();
    let sale_ends = sale_starts + 86_400;
    let token_amount = 500_000_0000000i128; // 500k tokens offered
    let price = 10_000i128; // 0.001 XLM per token → in stroops per token = 10_000? 
    // Simplify: price=2000 stroops (0.0002 XLM) per token; hard_cap = 100 XLM
    let price = 2000i128;
    let hard_cap = 100_0000000i128; // 100 XLM
    let min_buy = 1_0000000i128; // 1 XLM

    let sid = sale_client.create_sale(
        &dev, &token, &token_amount, &price, &hard_cap, &min_buy, &sale_starts, &sale_ends,
    );
    assert_eq!(sid, 0);

    // Sale contract must hold full allocation
    assert_eq!(token_client.balance(&sale_id_addr), token_amount);

    // buyer1 buys 50 XLM → 50/0.0002 = 250,000 tokens
    let xlm_buy = 50_0000000i128;
    sale_client.buy(&buyer1, &sid, &xlm_buy);
    let expected1 = xlm_buy * price / 10_000_000;
    assert_eq!(sale_client.get_purchase(&sid, &buyer1), expected1);
    assert_eq!(sale_client.get_sale(&sid).raised, xlm_buy);

    // buyer2 buys 30 XLM
    let xlm_buy2 = 30_0000000i128;
    sale_client.buy(&buyer2, &sid, &xlm_buy2);

    // Finalize after end
    env.ledger().with_mut(|l| l.timestamp = sale_ends + 1);
    sale_client.finalize(&sid);

    // Fee = 1.5% of 80 XLM = 1.2 XLM; dev gets 78.8 XLM
    let total_raised = xlm_buy + xlm_buy2;
    let fee = total_raised * 150 / 10_000;
    assert_eq!(native_client.balance(&fee_recipient), fee);
    assert_eq!(native_client.balance(&dev), total_raised - fee);

    // Buyers claim tokens
    sale_client.claim(&buyer1, &sid);
    assert_eq!(token_client.balance(&buyer1), expected1 + 100_0000000); // had 100 minted
    sale_client.claim(&buyer2, &sid);

    // Allocation drained correctly: contract holds remainder (unsold tokens stay with contract)
    let sold = expected1 + (xlm_buy2 * price / 10_000_000);
    assert_eq!(token_client.balance(&sale_id_addr), token_amount - sold);
}

#[test]
#[should_panic(expected = "dev not registered")]
fn test_unregistered_dev_cannot_create_sale() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let registry_id = env.register(StubRegistry, ());
    let registry_client = RegClientX::new(&env, &registry_id);
    registry_client.init(&admin);

    let token = create_token(&env, &token_admin);
    let native = create_token(&env, &token_admin);

    let sale_id_addr = env.register(crate::TokenSale, ());
    let sale_client = TokenSaleClient::new(&env, &sale_id_addr);
    sale_client.init(&admin, &registry_id, &native, &150, &fee_recipient);

    let t = env.ledger().timestamp();
    sale_client.create_sale(
        &stranger, &token, &100_0000000, &2000, &10_0000000, &1_0000000, &t, &(t + 100),
    );
}

// ---- Stub registry (mirrors agent-registry interface used by TokenSale) ----
#[contract]
pub struct StubRegistry;

#[contractimpl]
impl StubRegistry {
    pub fn init(env: Env, admin: Address) {
        admin.require_auth();
    }
    pub fn register(env: Env, owner: Address, operator: Address, metadata_uri: String) -> u64 {
        owner.require_auth();
        let id: u64 = env.storage().persistent().get(&CKey::N).unwrap_or(0) + 1;
        env.storage().persistent().set(&CKey::N, &id);
        id
    }
    pub fn agents_of(env: Env, owner: Address) -> Vec<u64> {
        let n: u64 = env.storage().persistent().get(&CKey::N).unwrap_or(0);
        let mut v = Vec::new(&env);
        if n > 0 { v.push_back(1); }
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
pub enum CKey { N }
