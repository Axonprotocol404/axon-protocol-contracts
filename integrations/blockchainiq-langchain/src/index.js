// AXON x Blockchain IQ — LangChain tools for the Blockchain IQ x402 API.
// Paid calls settle in USDC on Base (eip155:8453) via the x402 v2 standard.
import { tool } from "@langchain/core/tools";
import { z } from "zod";
import { privateKeyToAccount } from "viem/accounts";
import { wrapFetchWithPayment, x402Client, decodePaymentResponseHeader } from "@x402/fetch";
import { ExactEvmScheme } from "@x402/evm/exact/client";

export const BIQ_BASE_URL = "https://www.blockchainiq.co/api/x402/v1";

/**
 * Build a fetch that pays x402 402 responses in USDC on Base.
 * @param {object} opts
 * @param {string} opts.privateKey  0x-prefixed key of a dedicated, low-balance wallet
 * @param {number} [opts.maxUsdPerCall=0.1]  hard cap per single payment, in dollars
 */
export function createPayingFetch({ privateKey, maxUsdPerCall = 0.1 } = {}) {
  if (!privateKey) throw new Error("privateKey is required (set AGENT_PRIVATE_KEY)");
  const account = privateKeyToAccount(privateKey);
  const client = x402Client.fromConfig({
    schemes: [{ network: "eip155:*", client: new ExactEvmScheme(account) }],
    spendControls: { maxAmountPerPayment: maxUsdPerCall },
  });
  return { payFetch: wrapFetchWithPayment(fetch, client), address: account.address };
}

async function readPaid(res) {
  const header = res.headers.get("payment-response");
  const settlement = header ? decodePaymentResponseHeader(header) : null;
  const text = await res.text();
  let body;
  try { body = JSON.parse(text); } catch { body = { raw: text }; }
  if (!res.ok) {
    throw new Error(`Blockchain IQ returned HTTP ${res.status}: ${text.slice(0, 300)}`);
  }
  return { data: body.data ?? body, disclaimer: body.disclaimer, settlement };
}

/**
 * Create the LangChain tools.
 * @param {object} opts  same as createPayingFetch, plus optional baseUrl and onSettlement callback
 * @returns {import("@langchain/core/tools").StructuredToolInterface[]}
 */
export function createBlockchainIQTools(opts = {}) {
  const { baseUrl = BIQ_BASE_URL, onSettlement } = opts;
  const { payFetch } = createPayingFetch(opts);

  const call = async (path) => {
    const res = await payFetch(`${baseUrl}${path}`);
    const out = await readPaid(res);
    if (out.settlement && onSettlement) onSettlement(out.settlement, path);
    return JSON.stringify(out);
  };

  const brief = tool(async () => call("/brief"), {
    name: "blockchainiq_market_brief",
    description:
      "Paid ($0.01 USDC on Base via x402). Crypto market brief: BTC, ETH, SOL and XRP price with 24h/7d/30d change and rank, scheduled macro/crypto events in the next 48 hours, and short risk flags. Use as the first call before any trading or research decision. Takes no input.",
    schema: z.object({}),
  });

  const eventRisk = tool(
    async ({ days, minImportance }) => {
      const q = new URLSearchParams();
      if (days) q.set("days", String(days));
      if (minImportance) q.set("minImportance", String(minImportance));
      const qs = q.toString();
      return call(`/event-risk${qs ? `?${qs}` : ""}`);
    },
    {
      name: "blockchainiq_event_risk",
      description:
        "Paid ($0.05 USDC on Base via x402). Scheduled market-moving events for the next N days: US data releases (CPI, jobs, GDP, PCE), FOMC/ECB/BoJ decisions, Treasury auctions, and dated crypto/regulatory milestones, each with importance 1-3 and official source.",
      schema: z.object({
        days: z.number().int().min(1).max(90).optional().describe("Window in days, 1-90, default 7"),
        minImportance: z.union([z.literal(1), z.literal(2), z.literal(3)]).optional().describe("Lowest importance to include (1-3)"),
      }),
    }
  );

  const sample = tool(
    async () => {
      const res = await fetch(`${baseUrl}/sample`);
      return JSON.stringify(await readPaid(res));
    },
    {
      name: "blockchainiq_free_sample",
      description: "Free, no payment. Bitcoin network activity for the latest UTC day. Use to test connectivity.",
      schema: z.object({}),
    }
  );

  return [brief, eventRisk, sample];
}
