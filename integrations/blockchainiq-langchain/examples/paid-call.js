// Makes one real paid call ($0.01) to Blockchain IQ /brief through the LangChain tool
// and prints the settlement proof (transaction hash) from the PAYMENT-RESPONSE header.
// Usage: AGENT_PRIVATE_KEY=0x... node examples/paid-call.js
import { createBlockchainIQTools } from "../src/index.js";

const tools = createBlockchainIQTools({
  privateKey: process.env.AGENT_PRIVATE_KEY,
  maxUsdPerCall: 0.06,
  onSettlement: (s, path) => console.error(`settled ${path}:`, JSON.stringify(s)),
});
const brief = tools.find((t) => t.name === "blockchainiq_market_brief");
const out = JSON.parse(await brief.invoke({}));
console.log(JSON.stringify(out.data, null, 2));
console.log("\nSettlement proof:", JSON.stringify(out.settlement, null, 2));
if (!out.settlement?.transaction) process.exitCode = 1;
