import { beforeEach, describe, expect, it, mock } from "bun:test";
import type { ToolDefinition, ToolRunContext } from "@deepseek-ai/dsh-tools";

type ExecCallback = (error: Error | null, stdout: string, stderr: string) => void;

const execFileMock = mock((_file: string, _args: string[], _opts: unknown, _cb: ExecCallback) => {});

mock.module("node:child_process", () => ({ execFile: execFileMock }));

const { createReadTools } = await import("@/tools/read.js");

const exec = {} as ToolRunContext;
const tools = createReadTools("grove");

function tool(name: string): ToolDefinition {
  const found = tools.find(t => t.name === name);
  if (!found) throw new Error(`tool ${name} not registered`);
  return found;
}

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
};

describe("read-only tools", () => {
  it("registers the full read surface", () => {
    expect(tools.map(t => t.name).sort()).toEqual([
      "grove_list", "grove_next", "grove_packet", "grove_path", "grove_ready", "grove_show", "grove_status",
    ]);
  });

  it("grove_status runs `grove status`", async () => {
    await call("grove_status");
    expectArgv(["status"]);
  });

  it("grove_next runs `grove next`", async () => {
    await call("grove_next");
    expectArgv(["next"]);
  });

  it("grove_ready runs `grove ready`", async () => {
    await call("grove_ready");
    expectArgv(["ready"]);
  });

  it("grove_path runs `grove path`", async () => {
    await call("grove_path");
    expectArgv(["path"]);
  });

  it("grove_show forwards the node id", async () => {
    await call("grove_show", { id: "W-12" });
    expectArgv(["show", "W-12"]);
  });

  it("grove_packet forwards id and cone flags", async () => {
    await call("grove_packet", { id: "W-12", cone: true, coneDepth: 2, coneMax: 10 });
    expectArgv(["packet", "W-12", "--cone", "--cone-depth=2", "--cone-max=10"]);
  });

  it("grove_packet omits cone flags by default", async () => {
    await call("grove_packet", { id: "W-12" });
    expectArgv(["packet", "W-12"]);
  });

  it("grove_list forwards kind and filters", async () => {
    await call("grove_list", { kind: "q", status: "open", cynefin: "complicated" });
    expectArgv(["list", "q", "--status=open", "--cynefin=complicated"]);
  });

  it("returns stdout in the canonical value", async () => {
    await expect(call("grove_status")).resolves.toEqual({ stdout: "ok" });
  });

  it("propagates a CLI failure as a tool error with stderr", async () => {
    execFileMock.mockImplementation((_file: string, _args: string[], _opts: unknown, cb: ExecCallback) => {
      cb(new Error("exit 3"), "", "checksum mismatch");
    });
    await expect(call("grove_status")).rejects.toThrow("checksum mismatch");
  });
});
