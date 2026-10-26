// AXON x Blockchain IQ — Coinbase AgentKit action provider for the Blockchain IQ x402 API.
// Paid calls settle in USDC on Base (eip155:8453) via x402 v2, signed by the AgentKit wallet provider.
import { ActionProvider, EvmWalletProvider } from "@coinbase/agentkit";
import { wrapFetchWithPayment, x402Client, decodePaymentResponseHeader } from "@x402/fetch";
import { ExactEvmScheme } from "@x402/evm/exact/client";
import { z } from "zod";

export const BIQ_BASE_URL = "https://www.blockchainiq.co/api/x402/v1";

const EventRiskSchema = z.object({
  days: z.number().int().min(1).max(90).optional().describe("Window in days, 1-90, default 7"),
  minImportance: z.number().int().min(1).max(3).optional().describe("Lowest importance to include (1-3)"),
});
const EmptySchema = z.object({});

async function readPaid(res) {
  const header = res.headers.get("payment-response");
  const settlement = header ? decodePaymentResponseHeader(header) : null;
  const text = await res.text();
  let body;
  try { body = JSON.parse(text); } catch { body = { raw: text }; }
  if (!res.ok) throw new Error(`Blockchain IQ returned HTTP ${res.status}: ${text.slice(0, 300)}`);
  return { data: body.data ?? body, disclaimer: body.disclaimer, settlement };
}

/**
 * BlockchainIQActionProvider — three AgentKit actions backed by Blockchain IQ:
 *   blockchainiq_market_brief ($0.01), blockchainiq_event_risk ($0.05), blockchainiq_free_sample (free).
 * Payment is signed by whatever EvmWalletProvider the AgentKit instance uses (CDP, Viem, Privy, ...).
 */
export class BlockchainIQActionProvider extends ActionProvider {
  /**
   * @param {object} [opts]
   * @param {string} [opts.baseUrl]
   * @param {number} [opts.maxUsdPerCall=0.1] hard cap per single x402 payment, in dollars
   * @param {(settlement: object, path: string) => void} [opts.onSettlement]
   */
  constructor({ baseUrl = BIQ_BASE_URL, maxUsdPerCall = 0.1, onSettlement } = {}) {
    super("blockchainiq", []);
    this.baseUrl = baseUrl;
    this.maxUsdPerCall = maxUsdPerCall;
    this.onSettlement = onSettlement;
  }

  /** Base mainnet only: Blockchain IQ settles in USDC on eip155:8453. */
  supportsNetwork(network) {
    return network.protocolFamily === "evm" && String(network.chainId) === "8453";
  }

  async #payFetch(walletProvider) {
    if (!(walletProvider instanceof EvmWalletProvider) && typeof walletProvider.toSigner !== "function") {
      throw new Error("BlockchainIQActionProvider needs an EVM wallet provider on Base");
    }
    const signer = await walletProvider.toSigner();
    const client = x402Client.fromConfig({
      schemes: [{ network: "eip155:*", client: new ExactEvmScheme(signer) }],
      spendControls: { maxAmountPerPayment: this.maxUsdPerCall },
    });
    return wrapFetchWithPayment(fetch, client);
  }

  async #paidCall(walletProvider, path) {
    const payFetch = await this.#payFetch(walletProvider);
    const out = await readPaid(await payFetch(`${this.baseUrl}${path}`));
    if (out.settlement && this.onSettlement) this.onSettlement(out.settlement, path);
    return JSON.stringify(out);
  }

  getActions(walletProvider) {
    return [
      {
        name: "blockchainiq_market_brief",
        description:
          "Paid ($0.01 USDC on Base via x402). Crypto market brief: BTC, ETH, SOL and XRP price with 24h/7d/30d change and rank, scheduled macro/crypto events in the next 48 hours, and short risk flags. Use before any trading or research decision. Takes no input.",
        schema: EmptySchema,
        invoke: async () => this.#paidCall(walletProvider, "/brief"),
      },
      {
        name: "blockchainiq_event_risk",
        description:
          "Paid ($0.05 USDC on Base via x402). Scheduled market-moving events for the next N days: US data releases (CPI, jobs, GDP, PCE), FOMC/ECB/BoJ decisions, Treasury auctions, and dated crypto/regulatory milestones, each with importance 1-3 and an official source.",
        schema: EventRiskSchema,
        invoke: async (args) => {
          const { days, minImportance } = EventRiskSchema.parse(args ?? {});
          const q = new URLSearchParams();
          if (days) q.set("days", String(days));
          if (minImportance) q.set("minImportance", String(minImportance));
          const qs = q.toString();
          return this.#paidCall(walletProvider, `/event-risk${qs ? `?${qs}` : ""}`);
        },
      },
      {
        name: "blockchainiq_free_sample",
        description: "Free, no payment. Bitcoin network activity for the latest UTC day. Use to test connectivity.",
        schema: EmptySchema,
        invoke: async () => JSON.stringify(await readPaid(await fetch(`${this.baseUrl}/sample`))),
      },
    ];
  }
}

export const blockchainIQActionProvider = (opts) => new BlockchainIQActionProvider(opts);
