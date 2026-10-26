// No wallet needed: verifies the free sample and that /brief answers HTTP 402 with x402 v2 terms.
import { createBlockchainIQTools, BIQ_BASE_URL } from "../src/index.js";
const tools = createBlockchainIQTools({ privateKey: "0x" + "11".repeat(32) });
console.log((await tools.find((t) => t.name === "blockchainiq_free_sample").invoke({})).slice(0, 200));
const r = await fetch(`${BIQ_BASE_URL}/brief`);
const terms = JSON.parse(Buffer.from(r.headers.get("payment-required"), "base64").toString());
console.log("brief status", r.status, "x402Version", terms.x402Version, "amount", terms.accepts?.[0]?.amount, terms.accepts?.[0]?.network);
