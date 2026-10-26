// One real paid call ($0.01) to Blockchain IQ /brief through the AgentKit action provider,
// using AgentKit's ViemWalletProvider on Base mainnet. Prints the settlement proof (tx hash).
// Usage: AGENT_PRIVATE_KEY=0x... node examples/paid-call.js
import { AgentKit, ViemWalletProvider } from "@coinbase/agentkit";
import { createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { base } from "viem/chains";
import { blockchainIQActionProvider } from "../src/index.js";

// AgentKit fires a telemetry POST from wallet-provider constructors without catching failures;
// a rejected telemetry call must not crash the agent.
process.on("unhandledRejection", (e) => {
  if (String(e?.stack ?? e).includes("sendAnalyticsEvent")) return;
  console.error(e); process.exit(1);
});

const key = process.env.AGENT_PRIVATE_KEY;
if (!key) throw new Error("Set AGENT_PRIVATE_KEY to a throwaway wallet holding a few cents of USDC on Base");

const walletProvider = new ViemWalletProvider(
  createWalletClient({ account: privateKeyToAccount(key), chain: base, transport: http() })
);
const agentkit = await AgentKit.from({
  walletProvider,
  actionProviders: [blockchainIQActionProvider({ maxUsdPerCall: 0.06 })],
});

const brief = agentkit.getActions().find((a) => a.name === "blockchainiq_market_brief");
const out = JSON.parse(await brief.invoke({}));
console.log(JSON.stringify(out.data, null, 2));
console.log("\nPaid by:", walletProvider.getAddress());
console.log("Settlement proof:", JSON.stringify(out.settlement, null, 2));
if (!out.settlement?.transaction) process.exitCode = 1;
