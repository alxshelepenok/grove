import { tauriBridge } from "../integration/tauri-bridge.js";

export const wirePacket = (root, { reload } = {}) => {
  const forms = root.querySelector(".packet-forms");
  if (!forms) return null;
  const id = forms.dataset.packetId;

  const bind = (btnId, errId, run) => {
    const btn = forms.querySelector(`#${btnId}`);
    const errBox = forms.querySelector(`#${errId}`);
    btn?.addEventListener("click", async () => {
      errBox.textContent = "";
      errBox.hidden = true;
      btn.disabled = true;
      try {
        await run();
        await reload?.();
      } catch (e) {
        errBox.textContent = e?.message ?? String(e);
        errBox.hidden = false;
      } finally {
        btn.disabled = false;
      }
    });
  };

  bind("packet-evidence-submit", "packet-evidence-error", async () => {
    const text = forms.querySelector("#packet-evidence-text").value.trim();
    if (!text) throw new Error("evidence text is empty");
    await tauriBridge.invoke("grove_write", { cmd: "evidence", args: [id, text] });
  });

  bind("packet-status-submit", "packet-status-error", async () => {
    const status = forms.querySelector("#packet-status-select").value;
    await tauriBridge.invoke("grove_write", { cmd: "set", args: [id, `status=${status}`] });
  });

  const linkArgs = () => {
    const target = forms.querySelector("#packet-link-target").value.trim();
    if (!target) throw new Error("target id is empty");
    return [id, forms.querySelector("#packet-link-label").value, target];
  };

  bind("packet-link-add", "packet-link-error", async () => {
    await tauriBridge.invoke("grove_write", { cmd: "link", args: linkArgs() });
  });

  bind("packet-link-remove", "packet-link-error", async () => {
    await tauriBridge.invoke("grove_write", { cmd: "unlink", args: linkArgs() });
  });

  return null;
};
