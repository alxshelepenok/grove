import { defineTool } from "@deepseek-ai/dsh-tools";
import type { InferArgs, ParameterSchemaSpec, ToolDefinition } from "@deepseek-ai/dsh-tools";
import { runGrove } from "@/grove-cli.js";

interface ReadToolSpec<S extends ParameterSchemaSpec> {
  name: string;
  description: string;
  parameters: S;
  argv(args: InferArgs<S>): string[];
}

export const readTool = <S extends ParameterSchemaSpec>(spec: ReadToolSpec<S>, bin: string): ToolDefinition => {
  return defineTool({
    name: spec.name,
    description: spec.description,
    parameters: spec.parameters,
    output: {
      schema: {
        type: "object",
        additionalProperties: false,
        properties: {
          stdout: { type: "string", required: true, description: "Raw stdout of the grove command." },
        },
      },
      render: (_args, value) => [{ type: "text", text: value.stdout }],
    },
    async execute(args) {
      const stdout = await runGrove(spec.argv(args), { bin });
      return { stdout };
    },
  });
}

const NODE_KINDS = ["g", "w", "d", "q", "b", "t", "y", "a"] as const;

export const createReadTools = (bin: string) => {
  return [
    readTool({
      name: "grove_status",
      description: "Show Grove project status: work in progress, alignment triggers, and invariant notes. Read-only.",
      parameters: {},
      argv: () => ["status"],
    }, bin),
    readTool({
      name: "grove_next",
      description: "Propose the next work item (ready set intersected with the critical path) with its full execution packet. Read-only.",
      parameters: {},
      argv: () => ["next"],
    }, bin),
    readTool({
      name: "grove_ready",
      description: "List work items ready to start, critical-path first. Read-only.",
      parameters: {},
      argv: () => ["ready"],
    }, bin),
    readTool({
      name: "grove_path",
      description: "Show the critical path: the longest unfinished chain of blocks edges. Read-only.",
      parameters: {},
      argv: () => ["path"],
    }, bin),
    readTool({
      name: "grove_packet",
      description: "Emit the full execution packet for one work item: body, linked decisions, assumption chain, question outcomes, DoR breakdown. Read-only.",
      parameters: {
        id: { type: "string", required: true, description: "Work item id, e.g. W-12." },
        cone: { type: "boolean", description: "Append multi-hop structural context on blocks edges." },
        coneDepth: { type: "integer", description: "Cone BFS hops (default 4); only with cone." },
        coneMax: { type: "integer", description: "Cone node cap (default 50); only with cone." },
      },
      argv: (args) => {
        const argv = ["packet", args.id];
        if (args.cone) argv.push("--cone");
        if (args.coneDepth !== undefined) argv.push(`--cone-depth=${args.coneDepth}`);
        if (args.coneMax !== undefined) argv.push(`--cone-max=${args.coneMax}`);
        return argv;
      },
    }, bin),
    readTool({
      name: "grove_show",
      description: "Dump one Grove node record: header attrs and all prose fields. Read-only.",
      parameters: {
        id: { type: "string", required: true, description: "Node id, e.g. W-12, G-01, D-05." },
      },
      argv: (args) => ["show", args.id],
    }, bin),
    readTool({
      name: "grove_list",
      description: "List Grove nodes of one kind, optionally filtered by status or cynefin class. Read-only.",
      parameters: {
        kind: { type: "string", required: true, enum: NODE_KINDS, description: "Node kind: g, w, d, q, b, t, y, or a." },
        status: { type: "string", description: "Optional status filter, e.g. open, ready, done." },
        cynefin: { type: "string", enum: ["clear", "complicated", "complex", "chaotic"], description: "Optional cynefin filter." },
      },
      argv: (args) => {
        const argv = ["list", args.kind];
        if (args.status !== undefined) argv.push(`--status=${args.status}`);
        if (args.cynefin !== undefined) argv.push(`--cynefin=${args.cynefin}`);
        return argv;
      },
    }, bin),
  ];
}
