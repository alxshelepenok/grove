import { addDotIfNeeded } from "./common.js";

const getModalElements = () => {
  const modal = document.getElementById("alert-modal");
  return {
    modal,
    msgEl: document.getElementById("alert-modal-message"),
    titleEl: document.getElementById("alert-modal-title"),
    iconEl: document.getElementById("alert-modal-icon"),
    okBtn: document.getElementById("alert-modal-ok"),
    cancelBtn: document.getElementById("alert-modal-cancel"),
    inputEl: document.getElementById("alert-modal-input"),
  };
};

let confirmPendingResolve = null;
let promptPendingResolve = null;

const setModalContent = (message, isError, titleWhenNotError) => {
  const { msgEl, titleEl, iconEl } = getModalElements();
  if (!msgEl || !titleEl || !iconEl) return;

  msgEl.textContent = message;
  titleEl.textContent = isError ? "Something went wrong" : titleWhenNotError;

  const templateId = isError ? "alert-modal-icon-error" : "alert-modal-icon-warning";
  const tpl = document.getElementById(templateId);
  iconEl.replaceChildren(tpl?.content.cloneNode(true) ?? document.createDocumentFragment());
};

const openModal = (isConfirm) => {
  const { modal, cancelBtn } = getModalElements();
  if (!modal) return;

  modal.setAttribute("role", isConfirm ? "dialog" : "alertdialog");
  if (cancelBtn) cancelBtn.hidden = !isConfirm;
  modal.classList.add("active");
};

const closeModalOverlay = () => {
  const { modal } = getModalElements();
  modal?.classList.remove("active");
};

const finishConfirm = (result) => {
  closeModalOverlay();
  if (confirmPendingResolve) {
    confirmPendingResolve(result);
    confirmPendingResolve = null;
  }
};

const finishPrompt = (result) => {
  const { inputEl } = getModalElements();
  if (inputEl) inputEl.classList.add("hidden");
  closeModalOverlay();
  if (promptPendingResolve) {
    promptPendingResolve(result);
    promptPendingResolve = null;
  }
};

export const showAlert = (message, isError = false) => {
  const { modal, msgEl, inputEl } = getModalElements();
  if (!modal || !msgEl) return;

  if (confirmPendingResolve) {
    confirmPendingResolve(false);
    confirmPendingResolve = null;
  }

  if (inputEl) inputEl.classList.add("hidden");
  setModalContent(message || "Done.", isError, "System message");
  openModal(false);
};

export const showConfirm = (message, isError = false) => {
  return new Promise((resolve) => {
    const { modal, msgEl, inputEl } = getModalElements();
    if (!modal || !msgEl) {
      resolve(false);
      return;
    }

    if (confirmPendingResolve) {
      confirmPendingResolve(false);
      confirmPendingResolve = null;
    }

    if (inputEl) inputEl.classList.add("hidden");
    confirmPendingResolve = resolve;
    setModalContent(message || "Please confirm.", isError, "Confirm");
    openModal(true);
  });
};

export const showPrompt = (message, defaultValue = "") => {
  return new Promise((resolve) => {
    const { modal, msgEl, inputEl } = getModalElements();
    if (!modal || !msgEl || !inputEl) {
      resolve(null);
      return;
    }

    if (promptPendingResolve) {
      promptPendingResolve(null);
      promptPendingResolve = null;
    }

    promptPendingResolve = resolve;
    setModalContent(message || "Please enter:", false, "System message");
    inputEl.value = defaultValue;
    inputEl.classList.remove("hidden");
    openModal(true);
    setTimeout(() => inputEl.focus(), 0);
  });
};

export const showError = (message, details) => {
  if (details === undefined) {
    showAlert(addDotIfNeeded(message), true);
    return;
  }

  showAlert(`${message}: ${addDotIfNeeded(details)}`, true);
};

export const initAlertModal = () => {
  const { modal, okBtn, cancelBtn, inputEl } = getModalElements();
  if (!modal) return;

  okBtn?.addEventListener("click", () => {
    if (promptPendingResolve) finishPrompt(inputEl?.value ?? null);
    else if (confirmPendingResolve) finishConfirm(true);
    else closeModalOverlay();
  });

  cancelBtn?.addEventListener("click", () => {
    if (promptPendingResolve) finishPrompt(null);
    else finishConfirm(false);
  });

  document.getElementById("alert-modal-close-x")?.addEventListener("click", () => {
    if (promptPendingResolve) finishPrompt(null);
    else if (confirmPendingResolve) finishConfirm(false);
    else closeModalOverlay();
  });

  inputEl?.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (promptPendingResolve) finishPrompt(inputEl.value);
    }
  });
};
