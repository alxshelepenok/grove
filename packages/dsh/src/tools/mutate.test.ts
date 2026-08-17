import { beforeEach, describe, expect, it, mock } from "bun:test";
import type { ToolDefinition, ToolRunContext } from "@deepseek-ai/dsh-tools";

type ExecCallback = (error: Error | null, stdout: string, stderr: string) => void;

const execFileMock = mock((_file: string, _args: string[], _opts: unknown, _cb: ExecCallback) => {});

mock.module("node:child_process", () => ({ execFile: execFileMock }));

const { createMutateTools } = await import("@/tools/mutate.js");

const exec = {} as ToolRunContext;
const tools = createMutateTools("grove");

export const tool = (name: string): ToolDefinition => {
  const found = tools.find(t => t.name === name);
  if (!found) throw new Error(`tool ${name} not registered`);
  return found;
};

export const call = async (name: string, args: Record<string, unknown> = {}): Promise<unknown> => {
  return tool(name).execute(args, exec);
};

beforeEach(() => {
  execFileMock.mockReset();
  execFileMock.mockImplementation((_file: string, _args: string[], _opts: unknown, cb: ExecCallback) => {
    cb(null, "ok", "");
  });
});

export const expectArgv = (argv: string[]): void => {
  expect(execFileMock).toHaveBeenCalledWith("grove", argv, expect.objectContaining({}), expect.any(Function));
}

describe("mutating tools", () => {
  it("registers the full mutation surface", () => {
    expect(tools.map(t => t.name).sort()).toEqual([
      "grove_add", "grove_evidence", "grove_field", "grove_fitness", "grove_link", "grove_set",
    ]);
  });

  it("grove_add builds flags only for provided options", async () => {
    await call("grove_add", { kind: "w", title: "Add thing", type: "feature", cynefin: "clear", goals: "G-01" });
    expectArgv(["add", "w", "--title=Add thing", "--type=feature", "--cynefin=clear", "--goals=G-01"]);
  });

  it("grove_add forwards goal fitness flags", async () => {
    await call("grove_add", { kind: "g", title: "Outcome", area: "A-01", fitnessKind: "count", fitnessTarget: "4" });
    expectArgv(["add", "g", "--title=Outcome", "--area=A-01", "--fitness-kind=count", "--fitness-target=4"]);
  });

  it("grove_add omits absent options", async () => {
    await call("grove_add", { kind: "q", title: "Why?" });
    expectArgv(["add", "q", "--title=Why?"]);
  });

  it("grove_field forwards id, field, op, and value", async () => {
    await call("grove_field", { id: "W-12", field: "ac", op: "add", value: "User can sign in." });
    expectArgv(["field", "W-12", "ac", "add", "User can sign in."]);
  });

  it("grove_field clear carries no value", async () => {
    await call("grove_field", { id: "W-12", field: "ac", op: "clear" });
    expectArgv(["field", "W-12", "ac", "clear"]);
  });

  it("grove_set joins key and value", async () => {
    await call("grove_set", { id: "W-12", key: "status", value: "progress" });
    expectArgv(["set", "W-12", "status=progress"]);
  });

  it("grove_evidence forwards the evidence line", async () => {
    await call("grove_evidence", { id: "W-12", text: "tests green; abc123" });
    expectArgv(["evidence", "W-12", "tests green; abc123"]);
  });

  it("grove_fitness prefixes a non-negative delta with +", async () => {
    await call("grove_fitness", { id: "W-12", goal: "G-01", delta: 1 });
    expectArgv(["fitness", "W-12", "G-01", "+1"]);
  });

  it("grove_fitness keeps a negative delta as is", async () => {
    await call("grove_fitness", { id: "W-12", goal: "G-01", delta: -1 });
    expectArgv(["fitness", "W-12", "G-01", "-1"]);
  });

  it("grove_link forwards from, label, to", async () => {
    await call("grove_link", { from: "Q-03", label: "asks", to: "W-12" });
    expectArgv(["link", "Q-03", "asks", "W-12"]);
  });

  it("forwards an explicit session token as --session", async () => {
    await call("grove_set", { id: "W-12", key: "status", value: "done", session: "host:abc123" });
    expectArgv(["set", "W-12", "status=done", "--session=host:abc123"]);
  });

  it("propagates a CLI refusal as a tool error with stderr", async () => {
    execFileMock.mockImplementation((_file: string, _args: string[], _opts: unknown, cb: ExecCallback) => {
      cb(new Error("exit 4"), "", "DoR ≢ ⊤; see grove dor W-12");
    });
    await expect(call("grove_set", { id: "W-12", key: "status", value: "progress" })).rejects.toThrow("DoR ≢ ⊤");
  });
});
