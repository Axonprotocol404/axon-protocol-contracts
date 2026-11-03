//! AXON Protocol — AgentRegistry (Soroban)
//!
//! Verifiable on-chain identity for AI agents on Stellar.
//! Port of the Solidity AgentRegistry (ERC-721) to Soroban.
//!
//! Milestone 1 target for SCF #46. Work in progress — not audited.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    pub id: u64,
    pub owner: Address,      // human owner
    pub operator: Address,   // agent key authorized to act
    pub metadata_uri: String,
    pub active: bool,
    pub reputation: u32,
    pub task_count: u64,
    pub registered_at: u64,
}

#[contracttype]
pub enum DataKey {
    NextId,
    Agent(u64),           // agent_id -> Agent
    OwnerAgents(Address), // owner -> Vec<agent_id>
    Admin,
}

#[contract]
pub struct AgentRegistry;

#[contractimpl]
impl AgentRegistry {
    /// Initialize the registry with an admin.
    pub fn init(env: Env, admin: Address) {
        if env.storage().persistent().get(&DataKey::Admin).is_some() {
            panic!("already initialized");
        }
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::NextId, &1u64);
    }

    /// Register a new agent. `operator` is the agent's acting key.
    /// Returns the new agent id.
    pub fn register(env: Env, owner: Address, operator: Address, metadata_uri: String) -> u64 {
        owner.require_auth();

        let id: u64 = env.storage().persistent().get(&DataKey::NextId).unwrap_or(1);
        let agent = Agent {
            id,
            owner: owner.clone(),
            operator,
            metadata_uri,
            active: true,
            reputation: 0,
            task_count: 0,
            registered_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::Agent(id), &agent);

        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerAgents(owner.clone()))
            .unwrap_or(Vec::new(&env));
        list.push_back(id);
        env.storage().persistent().set(&DataKey::OwnerAgents(owner), &list);

        env.storage().persistent().set(&DataKey::NextId, &(id + 1));
        id
    }

    /// Owner rotates the operator (agent) key.
    pub fn set_operator(env: Env, agent_id: u64, new_operator: Address) {
        let mut agent = Self::_get(&env, agent_id);
        agent.owner.require_auth();
        agent.operator = new_operator;
        env.storage().persistent().set(&DataKey::Agent(agent_id), &agent);
    }

    /// Owner (de)activates an agent. Inactive agents fail `require_active`.
    pub fn set_active(env: Env, agent_id: u64, active: bool) {
        let mut agent = Self::_get(&env, agent_id);
        agent.owner.require_auth();
        agent.active = active;
        env.storage().persistent().set(&DataKey::Agent(agent_id), &agent);
    }

    /// Admin updates reputation (later: moved to a reputation oracle/protocol).
    pub fn set_reputation(env: Env, agent_id: u64, reputation: u32) {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        let mut agent = Self::_get(&env, agent_id);
        agent.reputation = reputation;
        env.storage().persistent().set(&DataKey::Agent(agent_id), &agent);
    }

    /// Views
    pub fn get_agent(env: Env, agent_id: u64) -> Agent {
        Self::_get(&env, agent_id)
    }

    pub fn agents_of(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerAgents(owner))
            .unwrap_or(Vec::new(&env))
    }

    /// Gate used by other AXON contracts (wallet factory, escrow, pad):
    /// revert unless `agent` is the operator of an active registered agent.
    pub fn require_active_operator(env: Env, agent: Address) -> u64 {
        let next: u64 = env.storage().persistent().get(&DataKey::NextId).unwrap_or(1);
        for id in 1..next {
            let a = Self::_get(&env, id);
            if a.operator == agent && a.active {
                return a.id;
            }
        }
        panic!("not an active agent operator");
    }

    fn _get(env: &Env, agent_id: u64) -> Agent {
        env.storage()
            .persistent()
            .get(&DataKey::Agent(agent_id))
            .unwrap_or_else(|| panic!("agent not found"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn register_and_gate() {
        let env = Env::default();
        let contract_id = env.register(AgentRegistry, ());
        let client = AgentRegistryClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let operator = Address::generate(&env);

        client.init(&admin);
        env.mock_all_auths();
        let id = client.register(&owner, &operator, &String::from_str(&env, "ipfs://agent1"));

        assert_eq!(client.get_agent(&id).owner, owner);
        assert_eq!(client.require_active_operator(&operator), id);

        client.set_active(&id, &false);
        let caught = std::panic::catch_unwind(|| client.require_active_operator(&operator));
        assert!(caught.is_err());
    }
}
