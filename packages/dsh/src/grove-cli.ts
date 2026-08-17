import { execFile } from "node:child_process";

export interface GroveRunOptions {
  bin?: string;
  cwd?: string;
  timeoutMs?: number;
}

export const runGrove = (args: string[], options: GroveRunOptions = {}): Promise<string> => {
  const bin = options.bin ?? "grove";
  return new Promise((resolve, reject) => {
    execFile(
      bin,
      args,
      { cwd: options.cwd, timeout: options.timeoutMs ?? 60_000, maxBuffer: 16 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error) {
          const detail = typeof stderr === "string" && stderr.trim().length > 0 ? stderr.trim() : error.message;
          reject(new Error(`grove ${args.join(" ")} failed: ${detail}`));
          return;
        }
        resolve(stdout);
      },
    );
  });
};
