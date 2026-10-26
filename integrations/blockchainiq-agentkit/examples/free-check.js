// No wallet needed. Registers the provider in AgentKit, runs the free sample action,
// and confirms /brief answers HTTP 402 with x402 v2 payment terms.
import { AgentKit, ViemWalletProvider } from "@coinbase/agentkit";
import { createWalletClient, http } from "viem";
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts";
import { base } from "viem/chains";
import { blockchainIQActionProvider, BIQ_BASE_URL } from "../src/index.js";

// AgentKit fires a telemetry POST from wallet-provider constructors without catching failures;
// a rejected telemetry call must not crash the agent.
process.on("unhandledRejection", (e) => {
  if (String(e?.stack ?? e).includes("sendAnalyticsEvent")) return;
  console.error(e); process.exit(1);
});

const walletProvider = new ViemWalletProvider(
  createWalletClient({ account: privateKeyToAccount(generatePrivateKey()), chain: base, transport: http() })
);
const agentkit = await AgentKit.from({ walletProvider, actionProviders: [blockchainIQActionProvider()] });
const names = agentkit.getActions().map((a) => a.name).filter((n) => n.startsWith("blockchainiq_"));
console.log("actions:", names.join(", "));

const sample = agentkit.getActions().find((a) => a.name === "blockchainiq_free_sample");
const s = JSON.parse(await sample.invoke({}));
console.log("free sample ok:", Object.keys(s.data ?? {}).slice(0, 6).join(", "));

const r = await fetch(`${BIQ_BASE_URL}/brief`);
const req = r.headers.get("payment-required");
const terms = req ? JSON.parse(Buffer.from(req, "base64").toString()) : null;
console.log("brief:", r.status, terms ? `x402 v${terms.x402Version}, ${terms.accepts?.[0]?.network}, amount ${terms.accepts?.[0]?.amount}` : "no terms");
if (names.length !== 3 || r.status !== 402 || !terms) process.exitCode = 1;
