//! AXON Protocol — AgentRegistry (Soroban)
//!
//! Verifiable on-chain identity for AI agents on Stellar.
//! Port of the Solidity AgentRegistry (ERC-721) to Soroban.
//!
//! Milestone 1 target for SCF #46. Work in progress — not audited.

#![no_std]

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
    Recorder(Address),    // authorized outcome recorder (e.g. the Escrow contract)
}

/// Reputation earned per successfully completed task (escrow released).
const REP_PER_COMPLETION: u32 = 10;
/// Reputation lost per failed task (saturating at 0).
const REP_FAILURE_PENALTY: u32 = 5;

#[contract]
pub struct AgentRegistry;

#[contractimpl]
impl AgentRegistry {
    /// Initialize the registry with an admin.
    pub fn init(env: Env, admin: Address) {
        if env.storage().persistent().get::<DataKey, Address>(&DataKey::Admin).is_some() {
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

    /// Admin sets reputation directly (manual override / bootstrap).
    /// Protocol contracts report outcomes via `record_outcome` instead.
    pub fn set_reputation(env: Env, agent_id: u64, reputation: u32) {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        let mut agent = Self::_get(&env, agent_id);
        agent.reputation = reputation;
        env.storage().persistent().set(&DataKey::Agent(agent_id), &agent);
    }

    /// Admin authorizes (or revokes) a contract that may report task
    /// outcomes — e.g. the SimpleEscrow contract. This is the only path by
    /// which escrow results reach an agent's reputation / task count.
    pub fn set_recorder(env: Env, recorder: Address, authorized: bool) {
        let admin: Address = env.storage().persistent().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if authorized {
            env.storage().persistent().set(&DataKey::Recorder(recorder), &true);
        } else {
            env.storage().persistent().remove(&DataKey::Recorder(recorder));
        }
    }

    /// Record the outcome of a task performed by `operator`.
    ///
    /// Caller must be an authorized recorder (see `set_recorder`) and must
    /// authorize the call — for a recorder contract, being the invoker of
    /// this call is its authorization, so reputation cannot be forged by
    /// third parties. On success the agent gains `REP_PER_COMPLETION`
    /// reputation; on failure it loses `REP_FAILURE_PENALTY` (floor 0).
    /// Either way `task_count` increments. Returns false (no state change)
    /// when `operator` is not a registered agent, so callers such as the
    /// escrow can settle payments to unregistered providers without
    /// reverting.
    pub fn record_outcome(env: Env, recorder: Address, operator: Address, success: bool) -> bool {
        if !env.storage().persistent().has(&DataKey::Recorder(recorder.clone())) {
            panic!("recorder not authorized");
        }
        recorder.require_auth();

        let next: u64 = env.storage().persistent().get(&DataKey::NextId).unwrap_or(1);
        for id in 1..next {
            let mut agent = Self::_get(&env, id);
            if agent.operator == operator {
                agent.task_count += 1;
                agent.reputation = if success {
                    agent.reputation.saturating_add(REP_PER_COMPLETION)
                } else {
                    agent.reputation.saturating_sub(REP_FAILURE_PENALTY)
                };
                env.storage().persistent().set(&DataKey::Agent(id), &agent);
                return true;
            }
        }
        false
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

    pub fn is_recorder(env: Env, recorder: Address) -> bool {
        env.storage().persistent().has(&DataKey::Recorder(recorder))
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
        let result = client.try_require_active_operator(&operator);
        assert!(result.is_err());
    }

    /// Setup helper: registry with one registered agent and one authorized
    /// recorder (a stand-in address for the Escrow contract). Returns
    /// (env, contract_id, agent_id, operator, recorder).
    fn setup() -> (Env, Address, u64, Address, Address) {
        let env = Env::default();
        let contract_id = env.register(AgentRegistry, ());
        let client = AgentRegistryClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        let operator = Address::generate(&env);
        let recorder = Address::generate(&env);

        client.init(&admin);
        env.mock_all_auths();
        let id = client.register(&owner, &operator, &String::from_str(&env, "ipfs://agent1"));
        client.set_recorder(&recorder, &true);
        (env, contract_id, id, operator, recorder)
    }

    #[test]
    fn record_outcome_success_updates_reputation_and_count() {
        let (env, contract_id, id, operator, recorder) = setup();
        let client = AgentRegistryClient::new(&env, &contract_id);

        assert!(client.record_outcome(&recorder, &operator, &true));
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 1);
        assert_eq!(agent.reputation, REP_PER_COMPLETION);

        assert!(client.record_outcome(&recorder, &operator, &true));
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 2);
        assert_eq!(agent.reputation, 2 * REP_PER_COMPLETION);
    }

    #[test]
    fn record_outcome_failure_penalizes_and_saturates() {
        let (env, contract_id, id, operator, recorder) = setup();
        let client = AgentRegistryClient::new(&env, &contract_id);

        // failure at reputation 0: stays 0, but the task still counts
        assert!(client.record_outcome(&recorder, &operator, &false));
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 1);
        assert_eq!(agent.reputation, 0);

        // success then failure: +10 then -5
        assert!(client.record_outcome(&recorder, &operator, &true));
        assert!(client.record_outcome(&recorder, &operator, &false));
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 3);
        assert_eq!(agent.reputation, REP_PER_COMPLETION - REP_FAILURE_PENALTY);
    }

    #[test]
    fn record_outcome_rejects_unauthorized_recorder() {
        let (env, contract_id, id, operator, recorder) = setup();
        let client = AgentRegistryClient::new(&env, &contract_id);
        let stranger = Address::generate(&env);

        // never authorized
        assert!(client.try_record_outcome(&stranger, &operator, &true).is_err());

        // authorized, then revoked
        assert!(client.is_recorder(&recorder));
        client.set_recorder(&recorder, &false);
        assert!(!client.is_recorder(&recorder));
        assert!(client.try_record_outcome(&recorder, &operator, &true).is_err());

        // no outcome was recorded through any of this
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 0);
        assert_eq!(agent.reputation, 0);
    }

    #[test]
    fn record_outcome_unknown_operator_is_noop() {
        let (env, contract_id, id, _operator, recorder) = setup();
        let client = AgentRegistryClient::new(&env, &contract_id);
        let ghost = Address::generate(&env);

        // unregistered operator: returns false, does not panic, no changes
        assert!(!client.record_outcome(&recorder, &ghost, &true));
        let agent = client.get_agent(&id);
        assert_eq!(agent.task_count, 0);
        assert_eq!(agent.reputation, 0);
    }
}
