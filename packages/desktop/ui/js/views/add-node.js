import { tauriBridge } from "../integration/tauri-bridge.js";

const KINDS_WITH_TAGS = ["w", "q", "b", "d", "g", "t"];

const csv = (value) =>
  String(value ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

const syncGoalDeltas = (container, goals) => {
  container.innerHTML = "";
  for (const gid of goals) {
    const row = document.createElement("div");
    row.className = "add-goal-delta-row";
    const label = document.createElement("span");
    label.className = "text-mono";
    label.textContent = gid;
    const input = document.createElement("input");
    input.type = "text";
    input.value = "+1";
    input.dataset.goal = gid;
    input.setAttribute("aria-label", `fitness delta for ${gid}`);
    row.append(label, input);
    container.append(row);
  }
};

const buildAddArgv = (val, kind) => {
  const argv = [kind, `--title=${val("add-node-title").trim()}`];
  const pushCsv = (flag, id) => {
    const items = csv(val(id));
    if (items.length) argv.push(`--${flag}=${items.join(",")}`);
  };
  switch (kind) {
    case "w":
      argv.push(`--type=${val("add-w-type")}`, `--cynefin=${val("add-w-cynefin")}`);
      pushCsv("goals", "add-w-goals");
      break;
    case "q":
      argv.push(`--cynefin=${val("add-q-cynefin")}`);
      pushCsv("targets", "add-q-targets");
      break;
    case "b":
      argv.push(`--cynefin=${val("add-b-cynefin")}`);
      pushCsv("tests", "add-b-tests");
      pushCsv("targets", "add-b-targets");
      break;
    case "d":
      pushCsv("supersedes", "add-d-supersedes");
      break;
    case "y":
      pushCsv("tags", "add-y-tags");
      pushCsv("surface", "add-y-surface");
      if (val("add-y-why").trim()) argv.push(`--why=${val("add-y-why").trim()}`);
      pushCsv("from", "add-y-from");
      break;
    case "g":
      argv.push(`--area=${val("add-g-area").trim()}`);
      if (val("add-g-fitness-kind"))
        argv.push(`--fitness-kind=${val("add-g-fitness-kind")}`);
      if (val("add-g-fitness-target").trim())
        argv.push(`--fitness-target=${val("add-g-fitness-target").trim()}`);
      break;
    case "t":
      break;
    case "a":
      pushCsv("surface", "add-a-surface");
      break;
  }
  return argv;
};

const validate = (val, kind) => {
  if (!val("add-node-title").trim()) return "title is required";
  if (kind === "y") {
    if (!csv(val("add-y-tags")).length) return "add y: tags are required";
    if (!csv(val("add-y-from")).length) return "add y: from is required";
  }
  if (kind === "g" && !val("add-g-area").trim()) return "add g: area is required";
  return null;
};

const runFollowUps = async (modal, kind, id) => {
  if (KINDS_WITH_TAGS.includes(kind)) {
    for (const tag of csv(modal.querySelector(`#add-${kind}-tags`)?.value)) {
      await tauriBridge.invoke("grove_write", {
        cmd: "field",
        args: [id, "tags", "add", tag],
      });
    }
  }
  if (kind === "w") {
    for (const input of modal.querySelectorAll("#add-w-goal-deltas input[data-goal]")) {
      await tauriBridge.invoke("grove_write", {
        cmd: "fitness",
        args: [id, input.dataset.goal, input.value.trim() || "+1"],
      });
    }
  }
};

export const wireAddModal = (root, { reload } = {}) => {
  const modal = root.querySelector("#add-node-modal");
  if (!modal) return null;
  const kindSelect = modal.querySelector("#add-node-kind");
  const errorBox = modal.querySelector("#add-node-error");
  const submitBtn = modal.querySelector("#add-node-submit");
  const deltasBox = modal.querySelector("#add-w-goal-deltas");
  const val = (id) => modal.querySelector(`#${id}`)?.value ?? "";

  const showError = (msg) => {
    errorBox.textContent = msg;
    errorBox.hidden = false;
  };
  const clearError = () => {
    errorBox.textContent = "";
    errorBox.hidden = true;
  };
  const close = () => modal.classList.remove("active");

  const syncKind = () => {
    for (const sec of modal.querySelectorAll("[data-add-kind]")) {
      sec.hidden = sec.dataset.addKind !== kindSelect.value;
    }
  };

  const resetForm = () => {
    modal.querySelector("#add-node-title").value = "";
    for (const input of modal.querySelectorAll("[data-add-kind] input[type='text']")) {
      input.value = "";
    }
    syncGoalDeltas(deltasBox, []);
  };

  kindSelect.addEventListener("change", syncKind);
  modal
    .querySelector("#add-w-goals")
    ?.addEventListener("input", (e) => syncGoalDeltas(deltasBox, csv(e.target.value)));
  modal.querySelector("#add-node-close")?.addEventListener("click", close);
  modal.querySelector("#add-node-cancel")?.addEventListener("click", close);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) close();
  });
  const onKey = (e) => {
    if (e.key === "Escape") close();
  };
  document.addEventListener("keydown", onKey);

  submitBtn.addEventListener("click", async () => {
    clearError();
    const kind = kindSelect.value;
    const problem = validate(val, kind);
    if (problem) {
      showError(problem);
      return;
    }
    submitBtn.disabled = true;
    try {
      const out = await tauriBridge.invoke("grove_write", {
        cmd: "add",
        args: buildAddArgv(val, kind),
      });
      const id = out.trim();
      await runFollowUps(modal, kind, id);
      close();
      resetForm();
      await reload?.();
    } catch (e) {
      showError(e?.message ?? String(e));
    } finally {
      submitBtn.disabled = false;
    }
  });

  return () => document.removeEventListener("keydown", onKey);
};
