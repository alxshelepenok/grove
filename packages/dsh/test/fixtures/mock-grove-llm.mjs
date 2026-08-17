import { CallId, LlmAdapter } from "@deepseek-ai/dsh-llm";

function* toolCall(name, args, id) {
  yield { type: "block-start", index: 0, blockType: "tool-call" };
  yield { type: "tool-call-delta", index: 0, id: CallId(id), name, argumentsDelta: args };
  yield { type: "block-end", index: 0, block: { type: "tool-call", id: CallId(id), name, arguments: args } };
  yield { type: "usage", usage: { inputTokens: 10, outputTokens: 5 } };
  yield { type: "finish", reason: { kind: "tool-calls" } };
}

class GroveSmokeAdapter extends LlmAdapter {
  async * stream(options) {
    const results = options.messages
      .flatMap(message => message.content)
      .filter(block => block.type === "tool-result");

    if (results.length === 0) {
      yield* toolCall("grove_status", "{}", "call-status");
      return;
    }
    if (results.length === 1) {
      yield* toolCall("grove_next", "{}", "call-next");
      return;
    }

    const texts = results
      .flatMap(block => block.content)
      .filter(block => block.type === "text")
      .map(block => block.text);
    const reply = ["grove smoke results:", ...texts].join("\n---\n");
    yield { type: "block-start", index: 0, blockType: "text" };
    yield { type: "text-delta", index: 0, text: reply };
    yield { type: "block-end", index: 0, block: { type: "text", text: reply } };
    yield { type: "usage", usage: { inputTokens: 10, outputTokens: reply.length } };
    yield { type: "finish", reason: { kind: "stop" } };
  }
}

export const name = "mock-grove-llm";
export const inject = ["llm"];

export const apply = (ctx) => {
  ctx.llm.registerAdapter(["mock"], new GroveSmokeAdapter());
};
