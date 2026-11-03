#![no_std]


use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype,
    token::TokenClient, Address, Env, String, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Registry,      // AgentRegistry contract — dev identity source
    FeeBps,        // protocol fee in basis points (e.g. 150 = 1.5%)
    FeeRecipient,
    NativeToken,   // native XLM wrapper (Stellar Asset Contract for XLM)
    SaleCount,
    Sale(u64),
    Purchase(u64, Address), // sale_id -> buyer -> purchased amount
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SaleError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    SaleNotFound = 3,
    SaleNotOpen = 4,
    SaleEnded = 5,
    SaleFull = 6,
    BelowMinimum = 7,
    NotVerifiedDev = 8,
    Unauthorized = 9,
    Overflow = 10,
    NothingToClaim = 11,
}

#[contract]
pub struct TokenSale;

#[contractimpl]
impl TokenSale {
    /// Init: admin, AgentRegistry (dev must be registered+active there), fee bps, fee recipient
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
        env.storage().persistent().set(&DataKey::SaleCount, &0u64);
    }

    /// Dev opens a sale. Dev must be an active agent in the Registry (anti-rug).
    /// Dev must pre-transfer the full token allocation to this contract.
    pub fn create_sale(
        env: Env,
        dev: Address,
        token: Address,
        token_amount: i128,
        price: i128,
        hard_cap: i128,
        min_buy: i128,
        starts_at: u64,
        ends_at: u64,
    ) -> u64 {
        dev.require_auth();
        let registry: Address = env
            .storage()
            .persistent()
            .get::<_, Address>(&DataKey::Registry)
            .expect("not initialized");

        // ANTI-RUG GATE: dev must be a registered, active agent
        let registry_client = RegClientX::new(&env, &registry);
        let agent_ids = registry_client.agents_of(&dev);
        if agent_ids.is_empty() {
            panic!("dev not registered");
        }
        let agent = registry_client.get_agent(&agent_ids.get(0).unwrap());
        if !agent.active {
            panic!("dev not active");
        }

        if price <= 0 || hard_cap <= 0 || token_amount <= 0 || ends_at <= env.ledger().timestamp() {
            panic!("invalid params");
        }
        // Token offered must cover hard_cap at this price
        if hard_cap
            .checked_div(price)
            .ok_or(SaleError::Overflow)
            .unwrap()
            > token_amount
        {
            panic!("token allocation below hard cap");
        }

        // Move the full allocation in — no partial funding, no rug
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&dev, &env.current_contract_address(), &token_amount);

        let sale_id: u64 = env.storage().persistent().get(&DataKey::SaleCount).unwrap();
        let sale = Sale {
            dev: dev.clone(),
            token: token.clone(),
            token_amount,
            price,
            hard_cap,
            min_buy,
            raised: 0,
            starts_at,
            ends_at,
            finalized: false,
        };
        env.storage().persistent().set(&DataKey::Sale(sale_id), &sale);
        env.storage()
            .persistent()
            .set(&DataKey::SaleCount, &(sale_id + 1));

        env.events().publish(
            (String::from_str(&env, "sale_created"),),
            (sale_id, dev, token, price, hard_cap),
        );
        sale_id
    }

    /// Buyer pays XLM; tokens escrowed in this contract are tracked for claim.
    pub fn buy(env: Env, buyer: Address, sale_id: u64, xlm_amount: i128) {
        buyer.require_auth();
        let mut sale: Sale = env
            .storage()
            .persistent()
            .get(&DataKey::Sale(sale_id))
            .expect("sale not found");

        let now = env.ledger().timestamp();
        if now < sale.starts_at {
            panic!("sale not open");
        }
        if now >= sale.ends_at {
            panic!("sale ended");
        }
        if xlm_amount < sale.min_buy {
            panic!("below minimum");
        }
        let new_raised = sale
            .raised
            .checked_add(xlm_amount)
            .ok_or(SaleError::Overflow)
            .unwrap();
        if new_raised > sale.hard_cap {
            panic!("over hard cap");
        }

        // Pull XLM from buyer via native token contract (set at init)
        let native_token: Address = env
            .storage()
            .persistent()
            .get(&DataKey::NativeToken)
            .expect("native token set at init");
        TokenClient::new(&env, &native_token).transfer(
            &buyer,
            &env.current_contract_address(),
            &xlm_amount,
        );

        let token_amount = xlm_amount
            .checked_mul(sale.price)
            .ok_or(SaleError::Overflow)
            .unwrap()
            / 10_000_000; // XLM stroops -> price in stroops per whole token

        sale.raised = new_raised;
        env.storage().persistent().set(&DataKey::Sale(sale_id), &sale);

        let mut bought: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Purchase(sale_id, buyer.clone()))
            .unwrap_or(0);
        bought = bought
            .checked_add(token_amount)
            .ok_or(SaleError::Overflow)
            .unwrap();
        env.storage()
            .persistent()
            .set(&DataKey::Purchase(sale_id, buyer.clone()), &bought);

        env.events().publish(
            (String::from_str(&env, "purchased"),),
            (sale_id, buyer.clone(), xlm_amount, token_amount),
        );
    }

    /// After sale ends: funds to dev minus protocol fee. Anyone can finalize.
    pub fn finalize(env: Env, sale_id: u64) {
        let mut sale: Sale = env
            .storage()
            .persistent()
            .get(&DataKey::Sale(sale_id))
            .expect("sale not found");
        if sale.finalized {
            panic!("already finalized");
        }
        if env.ledger().timestamp() < sale.ends_at {
            panic!("sale still running");
        }
        sale.finalized = true;
        env.storage().persistent().set(&DataKey::Sale(sale_id), &sale);

        let fee_bps: u32 = env.storage().persistent().get(&DataKey::FeeBps).unwrap_or(150);
        let fee = sale.raised * fee_bps as i128 / 10_000;
        let dev_amount = sale.raised - fee;

        let native_token: Address = env
            .storage()
            .persistent()
            .get(&DataKey::NativeToken)
            .expect("native token");
        let native = TokenClient::new(&env, &native_token);
        let fee_recipient: Address = env.storage().persistent().get(&DataKey::FeeRecipient).unwrap();
        if fee > 0 {
            native.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }
        native.transfer(&env.current_contract_address(), &sale.dev, &dev_amount);

        env.events().publish(
            (String::from_str(&env, "finalized"),),
            (sale_id, sale.raised, fee),
        );
    }

    /// Buyers claim their tokens after the sale ends.
    pub fn claim(env: Env, buyer: Address, sale_id: u64) {
        buyer.require_auth();
        let sale: Sale = env
            .storage()
            .persistent()
            .get(&DataKey::Sale(sale_id))
            .expect("sale not found");
        if env.ledger().timestamp() < sale.ends_at {
            panic!("sale still running");
        }
        let bought: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Purchase(sale_id, buyer.clone()))
            .unwrap_or(0);
        if bought <= 0 {
            panic!("nothing to claim");
        }
        env.storage().persistent().remove(&DataKey::Purchase(sale_id, buyer.clone()));
        TokenClient::new(&env, &sale.token).transfer(
            &env.current_contract_address(),
            &buyer,
            &bought,
        );
    }

    /// debug probe: is dev an active agent? (read-only)
    pub fn probe_dev(env: Env, dev: Address) -> u32 {
        let registry: Address = env.storage().persistent().get(&DataKey::Registry).expect("no reg");
        let rc = RegClientX::new(&env, &registry);
        let ids = rc.agents_of(&dev);
        ids.len() as u32
    }
    pub fn probe_agent(env: Env, dev: Address) -> u32 {
        let registry: Address = env.storage().persistent().get(&DataKey::Registry).expect("no reg");
        let rc = RegClientX::new(&env, &registry);
        let ids = rc.agents_of(&dev);
        let a = rc.get_agent(&ids.get(0).unwrap());
        if a.active { 1 } else { 2 }
    }

    // ---- read-only ----
    pub fn get_sale(env: Env, sale_id: u64) -> Sale {
        env.storage().persistent().get(&DataKey::Sale(sale_id)).expect("sale not found")
    }
    pub fn get_purchase(env: Env, sale_id: u64, buyer: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Purchase(sale_id, buyer))
            .unwrap_or(0)
    }
    pub fn sale_count(env: Env) -> u64 {
        env.storage().persistent().get(&DataKey::SaleCount).unwrap_or(0)
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sale {
    pub dev: Address,
    pub token: Address,
    pub token_amount: i128,
    pub price: i128,
    pub hard_cap: i128,
    pub min_buy: i128,
    pub raised: i128,
    pub starts_at: u64,
    pub ends_at: u64,
    pub finalized: bool,
}


#[cfg(test)]
mod test;

// ---- Local registry interface (mirrors on-chain AgentRegistry; no crate dep in wasm) ----
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
