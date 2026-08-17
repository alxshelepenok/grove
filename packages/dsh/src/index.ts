import type { Context } from "@deepseek-ai/cordis";
import { createMutateTools } from "@/tools/mutate.js";
import { createReadTools } from "@/tools/read.js";

export const name = "dsh-grove";
export const inject = ["tools"];

export interface Config {
  bin?: string;
}

export const apply = (ctx: Context, config?: Config): void => {
  const bin = config?.bin ?? "grove";
  for (const tool of [...createReadTools(bin), ...createMutateTools(bin)]) {
    ctx.tools.register(tool);
  }
};
