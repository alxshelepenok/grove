import { defineTool } from "@deepseek-ai/dsh-tools";
import type { InferArgs, ParameterSchemaSpec, ToolDefinition } from "@deepseek-ai/dsh-tools";
import { runGrove } from "@/grove-cli.js";

const SESSION_PARAM = {
  type: "string",
  description: "Grove session token forwarded as --session; defaults to the GROVE_SESSION environment variable inherited by the CLI.",
} as const;

interface MutateToolSpec<S extends ParameterSchemaSpec> {
  name: string;
  description: string;
  parameters: S;
  argv(args: InferArgs<S>): string[];
}

export const mutateTool = <S extends ParameterSchemaSpec>(spec: MutateToolSpec<S>, bin: string): ToolDefinition => {
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
      const argv = spec.argv(args);
      const session = (args as { session?: string }).session;
      if (session !== undefined) argv.push(`--session=${session}`);
      const stdout = await runGrove(argv, { bin });
      return { stdout };
    },
  });
};

export const createMutateTools = (bin: string) => {
  return [
    mutateTool({
      name: "grove_add",
      description: "Create a Grove node and print its assigned id. Required DoR fields for work items are added afterwards via grove_field.",
      parameters: {
        kind: { type: "string", required: true, enum: ["g", "w", "d", "q", "b", "t", "y", "a"], description: "Node kind." },
        title: { type: "string", required: true, description: "Node title; a concrete outcome for work items." },
        type: { type: "string", enum: ["feature", "refactor", "bug", "spike"], description: "Work item type (kind w)." },
        cynefin: { type: "string", enum: ["clear", "complicated", "complex", "chaotic"], description: "Cynefin class." },
        area: { type: "string", description: "Owning area A-NN (required for kind g)." },
        goals: { type: "string", description: "Comma-separated goal ids (kind w)." },
        theme: { type: "string", description: "Theme id T-NN (kind w)." },
        fitnessKind: { type: "string", enum: ["count", "ratio", "boolean", "metric", "manual"], description: "Structured fitness kind (kind g; some fitness value is required)." },
        fitnessTarget: { type: "string", description: "Structured fitness target (kind g)." },
        session: SESSION_PARAM,
      },
      argv: (args) => {
        const argv = ["add", args.kind, `--title=${args.title}`];
        if (args.type !== undefined) argv.push(`--type=${args.type}`);
        if (args.cynefin !== undefined) argv.push(`--cynefin=${args.cynefin}`);
        if (args.area !== undefined) argv.push(`--area=${args.area}`);
        if (args.goals !== undefined) argv.push(`--goals=${args.goals}`);
        if (args.theme !== undefined) argv.push(`--theme=${args.theme}`);
        if (args.fitnessKind !== undefined) argv.push(`--fitness-kind=${args.fitnessKind}`);
        if (args.fitnessTarget !== undefined) argv.push(`--fitness-target=${args.fitnessTarget}`);
        return argv;
      },
    }, bin),
    mutateTool({
      name: "grove_field",
      description: "Append to, remove from, or clear one prose field of a node (ac, hypothesis, evidence_strategy, outcome, vm, threshold, ...).",
      parameters: {
        id: { type: "string", required: true, description: "Node id, e.g. W-12." },
        field: { type: "string", required: true, description: "Field name, e.g. ac, hypothesis, outcome." },
        op: { type: "string", required: true, enum: ["add", "rm", "clear"], description: "Field operation." },
        value: { type: "string", description: "Entry text for add, or 1-based index for rm; unused for clear." },
        session: SESSION_PARAM,
      },
      argv: (args) => {
        const argv = ["field", args.id, args.field, args.op];
        if (args.value !== undefined) argv.push(args.value);
        return argv;
      },
    }, bin),
    mutateTool({
      name: "grove_set",
      description: "Set one guarded attribute of a node (status, cynefin, type, title, area, ...). Status transitions are enforced by CLI invariants.",
      parameters: {
        id: { type: "string", required: true, description: "Node id." },
        key: { type: "string", required: true, description: "Attribute key, e.g. status." },
        value: { type: "string", required: true, description: "New value, e.g. progress." },
        session: SESSION_PARAM,
      },
      argv: (args) => ["set", args.id, `${args.key}=${args.value}`],
    }, bin),
    mutateTool({
      name: "grove_evidence",
      description: "Append one falsifiable evidence line to a work item (test output, commit hash, build log).",
      parameters: {
        id: { type: "string", required: true, description: "Work item id, e.g. W-12." },
        text: { type: "string", required: true, description: "Evidence line." },
        session: SESSION_PARAM,
      },
      argv: (args) => ["evidence", args.id, args.text],
    }, bin),
    mutateTool({
      name: "grove_fitness",
      description: "Stage a per-goal fitness delta on a work item; applied atomically at status=done (I10).",
      parameters: {
        id: { type: "string", required: true, description: "Work item id." },
        goal: { type: "string", required: true, description: "Goal id, e.g. G-01." },
        delta: { type: "integer", required: true, description: "Signed delta; use 0 for enabling work." },
        session: SESSION_PARAM,
      },
      argv: (args) => ["fitness", args.id, args.goal, args.delta >= 0 ? `+${args.delta}` : `${args.delta}`],
    }, bin),
    mutateTool({
      name: "grove_link",
      description: "Create a typed edge between two nodes.",
      parameters: {
        from: { type: "string", required: true, description: "Source node id." },
        label: {
          type: "string",
          required: true,
          enum: ["blocks", "implements", "asks", "tests", "targets", "produces", "causes", "supersedes", "distills"],
          description: "Edge label.",
        },
        to: { type: "string", required: true, description: "Target node id." },
        session: SESSION_PARAM,
      },
      argv: (args) => ["link", args.from, args.label, args.to],
    }, bin),
  ];
};
