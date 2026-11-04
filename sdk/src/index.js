/**
 * AXON Protocol JS SDK — Stellar (Soroban)
 *
 * Thin client over the deployed AXON Soroban contracts:
 *  - AgentRegistry: register agents, verify operators
 *  - SimpleEscrow: open/release/cancel escrows
 *
 * Usage:
 *   import { AxonClient } from "@axon-protocol/sdk";
 *   const axon = new AxonClient({ network: "testnet", registry: REG_ID, escrow: ESCROW_ID });
 *   const agentId = await axon.registerAgent({ owner, operator, metadataUri });
 *   const ok = await axon.isActiveOperator(operator);
 */

import pkg from "@stellar/stellar-sdk";
const {
  rpc,
  Contract,
  Keypair,
  Networks,
  nativeToScVal,
  scValToNative,
  TransactionBuilder,
  Address,
  xdr,
} = pkg;

export const TESTNET_CONTRACTS = {
  registry: "CDLTKHIAAQPMJMGRGCUMVDSVWO672OSW3KEHUPEV5GFN3UXVCMIAHTFN",
  escrow: "CBT52U22SJW357F6Y3FGP3GY7IA4BT3XNLUKFSRAPEC732F425ALRL6L",
  tokenSale: "CAR2SSLRJVOWLCGHFUMSUAIX5IWVD43PNJUMRSP4K53RIIIRYT4F75LE",
};

const NETWORKS_MAP = {
  testnet: Networks.TESTNET,
  futurenet: Networks.FUTURENET,
  mainnet: Networks.PUBLIC,
};

const RPC_URLS = {
  testnet: "https://soroban-testnet.stellar.org",
  futurenet: "https://rpc-futurenet.stellar.org",
  mainnet: "https://soroban-rpc.stellar.org",
};

export class AxonClient {
  /**
   * @param {object} opts
   * @param {"testnet"|"futurenet"|"mainnet"} [opts.network="testnet"]
   * @param {string} [opts.registry] registry contract id (default: testnet deployment)
   * @param {string} [opts.escrow] escrow contract id
   * @param {string} [opts.rpcUrl] custom Soroban RPC
   */
  constructor(opts = {}) {
    this.network = opts.network ?? "testnet";
    this.networkPassphrase = NETWORKS_MAP[this.network];
    this.rpcUrl = opts.rpcUrl ?? RPC_URLS[this.network];
    this.server = new rpc.Server(this.rpcUrl, { allowHttp: this.network !== "mainnet" });
    this.registryId = opts.registry ?? (this.network === "testnet" ? TESTNET_CONTRACTS.registry : undefined);
    this.escrowId = opts.escrow ?? (this.network === "testnet" ? TESTNET_CONTRACTS.escrow : undefined);
    if (this.registryId) this.registry = new Contract(this.registryId);
    if (this.escrowId) this.escrow = new Contract(this.escrowId);
    this.tokenSaleId = opts.tokenSale ?? (this.network === "testnet" ? TESTNET_CONTRACTS.tokenSale : undefined);
    if (this.tokenSaleId) this.tokenSale = new Contract(this.tokenSaleId);
  }

  // ---------- helpers ----------

  async _read(contract, method, args = []) {
    const { Account } = pkg;
    const source = new Account(this._anyAccount(), "0"); // dummy source for read-only simulation
    const tx = new TransactionBuilder(source, {
      fee: "100",
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(30)
      .build();
    const sim = await this.server.simulateTransaction(tx);
    if (sim.error) throw new Error(`AXON ${method} failed: ${JSON.stringify(sim.error)}`);
    return scValToNative(sim.result.retval);
  }

  _anyAccount() {
    // read-only calls need a valid source account id — generate a throwaway one
    return Keypair.random().publicKey();
  }

  _i128(v) {
    return nativeToScVal(BigInt(v), { type: "i128" });
  }

  _u64(v) {
    return nativeToScVal(BigInt(v), { type: "u64" });
  }

  _addr(addr) {
    return nativeToScVal(addr, { type: "address" });
  }

  _str(s) {
    return nativeToScVal(s, { type: "string" });
  }

  // ---------- AgentRegistry ----------

  /** Build + sign + send an invocation with the given keypair. Returns submitted tx response. */
  async _invoke(contract, method, args, keypair) {
    const account = await this.server.getAccount(keypair.publicKey());
    const tx = new TransactionBuilder(account, {
      fee: "100000",
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(60)
      .build();
    const prepared = await this.server.prepareTransaction(tx);
    prepared.sign(keypair);
    const res = await this.server.sendTransaction(prepared);
    if (res.errorResult) throw new Error(`AXON ${method} failed: ${JSON.stringify(res.errorResult)}`);
    return res;
  }

  /**
   * Register a new agent. Requires the OWNER keypair.
   * @returns tx response (agent id from events or poll agentsOf)
   */
  registerAgent({ ownerKeypair, operator, metadataUri }) {
    return this._invoke(
      this.registry,
      "register",
      [this._addr(ownerKeypair.publicKey()), this._addr(operator), this._str(metadataUri)],
      ownerKeypair
    );
  }

  /** View: agent info by id */
  getAgent(agentId) {
    return this._read(this.registry, "get_agent", [this._u64(agentId)]);
  }

  /** View: all agent ids owned by `owner` */
  agentsOf(owner) {
    return this._read(this.registry, "agents_of", [this._addr(owner)]);
  }

  /** Gate: returns agent id if `operator` is an active agent operator, else throws */
  async isActiveOperator(operator) {
    return this._read(this.registry, "require_active_operator", [this._addr(operator)]);
  }

  /** Owner de/activates an agent */
  setAgentActive({ ownerKeypair, agentId, active }) {
    return this._invoke(this.registry, "set_active", [this._u64(agentId), nativeToScVal(active, { type: "bool" })], ownerKeypair);
  }

  /** Owner rotates the operator key */
  setOperator({ ownerKeypair, agentId, newOperator }) {
    return this._invoke(this.registry, "set_operator", [this._u64(agentId), this._addr(newOperator)], ownerKeypair);
  }

  // ---------- SimpleEscrow ----------

  /**
   * Open an escrow. Caller must first approve the escrow contract
   * to spend `amount` of `token` on behalf of client (standard SAC approve).
   */
  openEscrow({ clientKeypair, provider, token, amount }) {
    return this._invoke(
      this.escrow,
      "open",
      [this._addr(clientKeypair.publicKey()), this._addr(provider), this._addr(token), this._i128(amount)],
      clientKeypair
    );
  }

  /** Client releases the escrow to the provider (minus protocol fee) */
  releaseEscrow({ clientKeypair, escrowId }) {
    return this._invoke(this.escrow, "release", [this._u64(escrowId)], clientKeypair);
  }

  /** Mutual cancel (both parties sign — call twice, once per party) */
  cancelEscrow({ keypair, escrowId }) {
    return this._invoke(this.escrow, "mutual_cancel", [this._u64(escrowId)], keypair);
  }

  /** View: escrow state */
  getEscrow(escrowId) {
    return this._read(this.escrow, "get_escrow", [this._u64(escrowId)]);
  }
  // ---------- TokenSale (AXON Pad v0) ----------


  // read-only: get sale struct
  async getSale(saleId) {
    return this._read(this.tokenSale, 'get_sale', [nativeToScVal(BigInt(saleId), { type: 'u64' })]);
  }

  async saleCount() {
    return this._read(this.tokenSale, 'sale_count', []);
  }

  async getPurchase(saleId, buyer) {
    const args = [nativeToScVal(BigInt(saleId), { type: 'u64' }), new Address(buyer).toScVal()];
    return this._read(this.tokenSale, 'get_purchase', args);
  }

  // tx builders (return built XDR for wallet signing — no key handling here)
  async buildCreateSaleTx({ devKeypair, token, tokenAmount, price, hardCap, minBuy, startsAt, endsAt }) {
    const contract = this._saleContract();
    const params = [
      new xdr.Address(devKeypair.publicKey()).toScVal(),
      new xdr.Address(token).toScVal(),
      nativeToScVal(BigInt(tokenAmount), { type: 'i128' }),
      nativeToScVal(BigInt(price), { type: 'i128' }),
      nativeToScVal(BigInt(hardCap), { type: 'i128' }),
      nativeToScVal(BigInt(minBuy), { type: 'i128' }),
      nativeToScVal(startsAt, { type: 'u64' }),
      nativeToScVal(endsAt, { type: 'u64' }),
    ];
    const account = await this.server.getAccount(devKeypair.publicKey());
    const tx = new TransactionBuilder(account, {
      fee: '100000', networkPassphrase: this.networkPassphrase,
    })
      .addOperation(Operation.invokeContractFunction({
        contract, function: 'create_sale', args: params,
      }))
      .setTimeout(60).build();
    tx.sign(devKeypair);
    return tx.toXDR();
  }

}

export default AxonClient;
