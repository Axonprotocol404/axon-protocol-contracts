// A LangChain tool-calling agent that decides when to buy Blockchain IQ data.
// Usage: AGENT_PRIVATE_KEY=0x... OPENAI_API_KEY=sk-... node examples/agent.js
// (optional: npm i @langchain/openai)
import { createBlockchainIQTools } from "../src/index.js";

const { ChatOpenAI } = await import("@langchain/openai");
const tools = createBlockchainIQTools({ privateKey: process.env.AGENT_PRIVATE_KEY, maxUsdPerCall: 0.06 });
const llm = new ChatOpenAI({ model: "gpt-4o-mini" }).bindTools(tools);
const byName = Object.fromEntries(tools.map((t) => [t.name, t]));

const messages = [["human", "Give me a one-paragraph crypto market read for today, with any high-importance events in the next 48h."]];
let ai = await llm.invoke(messages);
messages.push(ai);
for (const call of ai.tool_calls ?? []) {
  messages.push(await byName[call.name].invoke(call));
}
ai = await llm.invoke(messages);
console.log(ai.content);
