import { createMutateTools } from "../target/tools/mutate.js";
import { createReadTools } from "../target/tools/read.js";

const tools = Object.fromEntries(
  [...createMutateTools("grove"), ...createReadTools("grove")].map(t => [t.name, t]),
);

export const call = async (name, args = {}) => {
  const { stdout } = await tools[name].execute(args, {});
  const line = stdout.trim();
  console.log(`${name} -> ${line}`);
  return line;
};

const area = await call("grove_add", { kind: "a", title: "Scratch area" });
const goal = await call("grove_add", {
  kind: "g", title: "Scratch goal", area, fitnessKind: "count", fitnessTarget: "1",
});
const work = await call("grove_add", {
  kind: "w", title: "Scratch work item", type: "feature", cynefin: "clear", goals: goal,
});

await call("grove_field", { id: work, field: "ac", op: "add", value: "The round trip reaches done." });
await call("grove_field", { id: work, field: "hypothesis", op: "add", value: "The wrapper forwards argv faithfully." });
await call("grove_field", { id: work, field: "evidence_strategy", op: "add", value: "This script output." });
await call("grove_fitness", { id: work, goal, delta: 1 });
await call("grove_evidence", { id: work, text: "round-trip.mjs completed; bun test green" });
await call("grove_set", { id: work, key: "status", value: "ready" });
await call("grove_set", { id: work, key: "status", value: "progress" });
await call("grove_set", { id: work, key: "status", value: "done" });
await call("grove_show", { id: goal });

console.log("ROUND_TRIP_OK");
