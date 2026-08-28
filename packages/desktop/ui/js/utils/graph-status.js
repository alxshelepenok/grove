export const KIND_INFO = {
  w: "Work",
  g: "Goal",
  q: "Question",
  b: "Assumption",
  d: "Decision",
  y: "Discovery",
  t: "Theme",
  a: "Area",
  root: "Project",
};

export const STATUS_VARIANTS = {
  g: { unverified: "warning", partial: "info", verified: "success", declined: "danger" },
  w: {
    proposed: "neutral",
    ready: "info",
    progress: "accent",
    done: "success",
    rejected: "danger",
    archived: "neutral",
  },
  q: { open: "warning", answered: "success", deferred: "neutral", dropped: "neutral" },
  b: {
    proposed: "neutral",
    testing: "info",
    validated: "success",
    invalidated_acceptable: "warning",
    invalidated_blocking: "danger",
  },
  t: { open: "info", done: "success" },
  y: { proposed: "warning", active: "success", stale: "danger", superseded: "neutral" },
  d: { proposed: "warning", accepted: "success", rejected: "danger", superseded: "neutral" },
};

export const KIND_DEFAULT_VARIANT = { t: "info" };

export const statusVariant = (n) =>
  STATUS_VARIANTS[n.kind]?.[n.status] ?? KIND_DEFAULT_VARIANT[n.kind] ?? "neutral";
