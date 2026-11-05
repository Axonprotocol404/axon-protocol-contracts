#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype,
    token::TokenClient, Address, Env, String, Vec,
};

/// Linear bonding curve: price = base_price + slope * supply
/// Tokens minted-on-demand are NOT used (no token authority); instead the dev
/// pre-deposits the full token allocation, and the curve governs pricing.
/// This keeps the contract permissionless for buyers while the identity gate
/// lives in the factory flow (same as TokenSale: launcher must be a verified agent).

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Registry,
    NativeToken,
    FeeBps,
    FeeRecipient,
    CurveCount,
    Curve(u64),
    Holding(u64, Address), // curve_id -> buyer -> tokens held
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CurveError {
    NotInitialized = 1,
    CurveNotFound = 2,
    CurveEnded = 3,
    NothingToSell = 4,
    Overflow = 5,
    Empty = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Curve {
    pub dev: Address,
    pub token: Address,
    pub token_balance: i128,   // tokens remaining in contract
    pub base_price: i128,      // stroops XLM at supply 0
    pub slope: i128,           // price increment per 1_000_000 tokens sold
    pub sold: i128,            // tokens sold so far
    pub ends_at: u64,
    pub finalized: bool,
}

#[contract]
pub struct BondingCurve;

#[contractimpl]
impl BondingCurve {
    pub fn init(env: Env, admin: Address, registry: Address, native_token: Address, fee_bps: u32, fee_recipient: Address) {
        if env.storage().persistent().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Registry, &registry);
        env.storage().persistent().set(&DataKey::NativeToken, &native_token);
        env.storage().persistent().set(&DataKey::FeeBps, &fee_bps);
        env.storage().persistent().set(&DataKey::FeeRecipient, &fee_recipient);
        env.storage().persistent().set(&DataKey::CurveCount, &0u64);
    }

    /// Launch a curve. Dev must be a registered, active agent (identity gate, same as TokenSale).
    /// Full token allocation is transferred in at launch — the curve only governs price discovery.
    pub fn create_curve(
        env: Env,
        dev: Address,
        token: Address,
        token_amount: i128,
        base_price: i128,
        slope: i128,
        ends_at: u64,
    ) -> u64 {
        dev.require_auth();
        let registry: Address = env.storage().persistent().get(&DataKey::Registry).expect("not initialized");
        let rc = RegClientX::new(&env, &registry);
        let ids = rc.agents_of(&dev);
        if ids.is_empty() { panic!("dev not registered"); }
        let agent = rc.get_agent(&ids.get(0).unwrap());
        if !agent.active { panic!("dev not active"); }

        if token_amount <= 0 || base_price <= 0 || slope < 0 || ends_at <= env.ledger().timestamp() {
            panic!("invalid params");
        }

        TokenClient::new(&env, &token).transfer(&dev, &env.current_contract_address(), &token_amount);

        let id: u64 = env.storage().persistent().get(&DataKey::CurveCount).unwrap_or(0);
        let curve = Curve {
            dev: dev.clone(),
            token: token.clone(),
            token_balance: token_amount,
            base_price,
            slope,
            sold: 0,
            ends_at,
            finalized: false,
        };
        env.storage().persistent().set(&DataKey::Curve(id), &curve);
        env.storage().persistent().set(&DataKey::CurveCount, &(id + 1));
        env.events().publish((String::from_str(&env, "curve_created"),), (id, dev, token, token_amount));
        id
    }

    /// Current buy price in stroops XLM per 1 token unit, given tokens sold so far.
    fn price_at(sold: i128, base_price: i128, slope: i128) -> i128 {
        base_price + (sold / 1_000_000) * slope
    }

    /// Integral of price over [sold, sold+amount) — exact for step function.
    fn cost_between(from: i128, to: i128, base_price: i128, slope: i128) -> i128 {
        // sum over 1M-token buckets
        let mut total: i128 = 0;
        let mut cur = from;
        while cur < to {
            let bucket_end = ((cur / 1_000_000) + 1) * 1_000_000;
            let seg_end = if bucket_end < to { bucket_end } else { to };
            let seg_len = seg_end - cur;
            total += Self::price_at(cur, base_price, slope) * seg_len;
            cur = seg_end;
        }
        total
    }

    /// Buy `token_amount` tokens at the current curve price; pays XLM.
    pub fn buy(env: Env, buyer: Address, curve_id: u64, token_amount: i128, max_cost: i128) {
        buyer.require_auth();
        let mut curve: Curve = env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found");
        if curve.finalized || env.ledger().timestamp() >= curve.ends_at { panic!("curve ended"); }
        if token_amount <= 0 { panic!("invalid amount"); }
        if token_amount > curve.token_balance { panic!("insufficient supply"); }

        let cost = Self::cost_between(curve.sold, curve.sold + token_amount, curve.base_price, curve.slope);
        if cost > max_cost { panic!("slippage: cost exceeds max_cost"); }

        let native: Address = env.storage().persistent().get(&DataKey::NativeToken).expect("native");
        TokenClient::new(&env, &native).transfer(&buyer, &env.current_contract_address(), &cost);

        // deliver tokens immediately (buyer holds them; sell transfers back)
        TokenClient::new(&env, &curve.token).transfer(&env.current_contract_address(), &buyer, &token_amount);

        curve.sold += token_amount;
        curve.token_balance -= token_amount;
        env.storage().persistent().set(&DataKey::Curve(curve_id), &curve);

        let held: i128 = env.storage().persistent().get(&DataKey::Holding(curve_id, buyer.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Holding(curve_id, buyer.clone()), &(held + token_amount));

        env.events().publish((String::from_str(&env, "bought"),), (curve_id, buyer.clone(), token_amount, cost));
    }

    /// Sell tokens back into the curve at the current sell price (refund at price of remaining supply).
    pub fn sell(env: Env, seller: Address, curve_id: u64, token_amount: i128, min_refund: i128) {
        seller.require_auth();
        let mut curve: Curve = env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found");
        if curve.finalized { panic!("curve ended"); }
        let held: i128 = env.storage().persistent().get(&DataKey::Holding(curve_id, seller.clone())).unwrap_or(0);
        if token_amount <= 0 || token_amount > held { panic!("insufficient holding"); }

        // refund = cost between (sold - amount, sold) — reverse integral
        let refund = Self::cost_between(curve.sold - token_amount, curve.sold, curve.base_price, curve.slope);
        if refund < min_refund { panic!("slippage: refund below min_refund"); }

        let fee_bps: u32 = env.storage().persistent().get(&DataKey::FeeBps).unwrap_or(150);
        let fee = refund * fee_bps as i128 / 10_000;
        let net = refund - fee;

        let native: Address = env.storage().persistent().get(&DataKey::NativeToken).expect("native");
        let native_client = TokenClient::new(&env, &native);
        // tokens return to the contract's tradable pool
        let token_client = TokenClient::new(&env, &curve.token);
        token_client.transfer(&seller, &env.current_contract_address(), &token_amount);

        let fee_recipient: Address = env.storage().persistent().get(&DataKey::FeeRecipient).unwrap();
        if fee > 0 { native_client.transfer(&env.current_contract_address(), &fee_recipient, &fee); }
        native_client.transfer(&env.current_contract_address(), &seller, &net);

        curve.sold -= token_amount;
        curve.token_balance += token_amount;
        env.storage().persistent().set(&DataKey::Curve(curve_id), &curve);
        env.storage().persistent().set(&DataKey::Holding(curve_id, seller.clone()), &(held - token_amount));

        env.events().publish((String::from_str(&env, "sold"),), (curve_id, seller.clone(), token_amount, net));
    }

    /// After the curve ends: unsold tokens return to dev, XLM raised is paid out minus protocol fee.
    pub fn finalize(env: Env, curve_id: u64) {
        let mut curve: Curve = env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found");
        if curve.finalized { panic!("already finalized"); }
        if env.ledger().timestamp() < curve.ends_at { panic!("curve still running"); }
        curve.finalized = true;
        env.storage().persistent().set(&DataKey::Curve(curve_id), &curve);

        // XLM held by the contract = integral of price over [0, sold] minus any refunds+fees paid out.
        // We track it implicitly: total_in = cost_between(0, sold). Refunds were paid out at sell-time.
        // For payout we send the dev the current contract XLM balance minus fee share. Simplest correct approach:
        // fee = fee_bps applied to (balance - refund_reserve). Since every held XLM came from buys net of sells,
        // dev gets the full remaining balance minus the protocol's fee share of gross volume.
        let native: Address = env.storage().persistent().get(&DataKey::NativeToken).expect("native");
        let native_client = TokenClient::new(&env, &native);
        let bal = native_client.balance(&env.current_contract_address());

        let fee_bps: u32 = env.storage().persistent().get(&DataKey::FeeBps).unwrap_or(150);
        let fee = bal * fee_bps as i128 / 10_000;
        let dev_amount = bal - fee;

        let fee_recipient: Address = env.storage().persistent().get(&DataKey::FeeRecipient).unwrap();
        if fee > 0 { native_client.transfer(&env.current_contract_address(), &fee_recipient, &fee); }
        if dev_amount > 0 { native_client.transfer(&env.current_contract_address(), &curve.dev, &dev_amount); }

        // unsold tokens back to dev
        if curve.token_balance > 0 {
            TokenClient::new(&env, &curve.token).transfer(&env.current_contract_address(), &curve.dev, &curve.token_balance);
            curve.token_balance = 0;
            env.storage().persistent().set(&DataKey::Curve(curve_id), &curve);
        }

        env.events().publish((String::from_str(&env, "finalized"),), (curve_id, curve.sold, fee));
    }

    // ---- read-only ----
    pub fn get_curve(env: Env, curve_id: u64) -> Curve {
        env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found")
    }
    pub fn current_price(env: Env, curve_id: u64) -> i128 {
        let c: Curve = env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found");
        Self::price_at(c.sold, c.base_price, c.slope)
    }
    pub fn quote_buy(env: Env, curve_id: u64, token_amount: i128) -> i128 {
        let c: Curve = env.storage().persistent().get(&DataKey::Curve(curve_id)).expect("curve not found");
        Self::cost_between(c.sold, c.sold + token_amount, c.base_price, c.slope)
    }
    pub fn get_holding(env: Env, curve_id: u64, who: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Holding(curve_id, who)).unwrap_or(0)
    }
    pub fn curve_count(env: Env) -> u64 {
        env.storage().persistent().get(&DataKey::CurveCount).unwrap_or(0)
    }
}

// ---- Local registry interface (no crate dep in wasm — see TokenSale gotcha) ----
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RAgent {
    pub id: u64,
    pub owner: Address,
    pub operator: Address,
    pub metadata_uri: String,
    pub active: bool,
    pub reputation: u32,
    pub task_count: u64,
    pub registered_at: u64,
}

#[contractclient(name = "RegClientX")]
pub trait RegistryX {
    fn init(env: Env, admin: Address);
    fn register(env: Env, owner: Address, operator: Address, metadata_uri: String) -> u64;
    fn agents_of(env: Env, owner: Address) -> Vec<u64>;
    fn get_agent(env: Env, agent_id: u64) -> RAgent;
}

#[cfg(test)]
mod test;
