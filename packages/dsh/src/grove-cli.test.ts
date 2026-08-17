import { beforeEach, describe, expect, it, mock } from "bun:test";

type ExecCallback = (error: Error | null, stdout: string, stderr: string) => void;

const execFileMock = mock((_file: string, _args: string[], _opts: unknown, _cb: ExecCallback) => {});

mock.module("node:child_process", () => ({ execFile: execFileMock }));

const { runGrove } = await import("@/grove-cli.js");

export const stub = (outcome: { stdout?: string; stderr?: string; error?: Error | null }): void => {
  execFileMock.mockImplementation((_file: string, _args: string[], _opts: unknown, cb: ExecCallback) => {
    cb(outcome.error ?? null, outcome.stdout ?? "", outcome.stderr ?? "");
  });
};

beforeEach(() => {
  execFileMock.mockReset();
});

describe("runGrove", () => {
  it("resolves with raw stdout on exit code 0", async () => {
    stub({ stdout: "graph ok\n" });
    await expect(runGrove(["status"])).resolves.toBe("graph ok\n");
    expect(execFileMock).toHaveBeenCalledWith("grove", ["status"], expect.objectContaining({}), expect.any(Function));
  });

  it("uses the configured binary", async () => {
    stub({ stdout: "x" });
    await runGrove(["ready"], { bin: "/opt/grove/bin/grove" });
    expect(execFileMock).toHaveBeenCalledWith("/opt/grove/bin/grove", ["ready"], expect.objectContaining({}), expect.any(Function));
  });

  it("rejects with trimmed stderr on non-zero exit", async () => {
    stub({ error: new Error("exit 1"), stderr: "  DoR ≢ ⊤; see grove dor W-12\n" });
    await expect(runGrove(["set", "W-12", "status=progress"])).rejects.toThrow("DoR ≢ ⊤");
  });

  it("falls back to the error message when stderr is empty", async () => {
    stub({ error: new Error("spawn grove ENOENT"), stderr: "" });
    await expect(runGrove(["status"])).rejects.toThrow("spawn grove ENOENT");
  });
});
