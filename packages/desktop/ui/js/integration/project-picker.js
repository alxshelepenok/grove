import { tauriBridge } from "./tauri-bridge.js";
import { openCommandMenu } from "../utils/command-menu.js";

let activeMenu = null;

const middleTruncate = (value, max = 56) => {
  const s = String(value ?? "");
  if (s.length <= max) return s;
  const head = Math.ceil((max - 3) / 2);
  const tail = max - 3 - head;
  return `${s.slice(0, head)}...${s.slice(s.length - tail)}`;
};

const errText = (e) => String(e?.message ?? e);

const projectChanged = () =>
  window.dispatchEvent(new Event("grove:project-changed"));

const pickDirectory = async () => {
  try {
    return await tauriBridge.invoke("grove_pick_directory");
  } catch {
    return null;
  }
};

const projectAvatar = async (name) => {
  try {
    return await tauriBridge.invoke("grove_project_avatar", { name });
  } catch {
    return undefined;
  }
};

export const openProjectPicker = async (mode = "default", opts = {}) => {
  if (!tauriBridge.available) return;
  if (activeMenu?.isOpen()) return;
  let menu;

  const fetchList = async () => {
    try {
      return await tauriBridge.invoke("grove_projects_list");
    } catch {
      return null;
    }
  };

  const submitOpen = async ({ path }) => {
    const p = String(path ?? "").trim();
    if (!p) return "enter a project path";
    try {
      await tauriBridge.invoke("grove_project_open", { path: p });
    } catch (e) {
      return errText(e);
    }
    projectChanged();
    return null;
  };

  const submitCreate = async ({ path, name }) => {
    const p = String(path ?? "").trim();
    if (!p) return "enter a project path";
    try {
      await tauriBridge.invoke("grove_project_create", {
        path: p,
        name: String(name ?? "").trim(),
      });
    } catch (e) {
      return errText(e);
    }
    projectChanged();
    return null;
  };

  const closeProject = async () => {
    try {
      await tauriBridge.invoke("grove_project_close");
    } catch (e) {
      menu.showError(errText(e));
      return true;
    }
    projectChanged();
    return null;
  };

  const buildSections = async (list) => {
    const currentPath = list?.current?.path ?? null;
    const actions = [
      {
        id: "open",
        icon: "folder-open",
        label: "Open project...",
        action: () =>
          menu.showForm({
            icon: "folder-open",
            fields: [
              { key: "path", placeholder: "Path to an existing project..." },
            ],
            hint: "The folder must contain .grove/state.lock.",
            browse: pickDirectory,
            submit: submitOpen,
          }),
      },
      {
        id: "create",
        icon: "add",
        label: "Create project...",
        action: () =>
          menu.showForm({
            icon: "add",
            fields: [
              { key: "path", placeholder: "Path for the new project..." },
              {
                key: "name",
                placeholder: "Project name (optional)",
                optional: true,
              },
            ],
            hint: "Initializes .grove/state.lock at the path; the name defaults to the folder name.",
            browse: pickDirectory,
            submit: submitCreate,
          }),
      },
    ];
    if (currentPath) {
      actions.push({
        id: "close",
        icon: "cancel",
        label: "Close current project",
        action: closeProject,
      });
    }

    const sections = [{ title: "Actions", items: actions }];
    if (window.workspace?.appEnv === "development" && opts.navigate) {
      sections.push({
        title: "Development actions",
        items: [
          {
            id: "debug-view",
            icon: "bug",
            label: "Debug view",
            action: async () => {
              await opts.navigate("debug");
              return null;
            },
          },
        ],
      });
    }

    const recents = await Promise.all(
      (list?.recents ?? []).map(async (r) => ({
        id: `recent:${r.path}`,
        icon: "tree",
        avatar: await projectAvatar(String(r.name ?? "")),
        label: r.name,
        secondary: middleTruncate(r.path),
        badge: r.path === currentPath ? "current" : undefined,
        action: async () => {
          try {
            await tauriBridge.invoke("grove_project_open", { path: r.path });
          } catch (e) {
            menu.showError(errText(e));
            return true;
          }
          projectChanged();
          return null;
        },
        remove: async () => {
          let next = null;
          try {
            next = await tauriBridge.invoke("grove_project_remove", {
              path: r.path,
            });
          } catch (e) {
            menu.showError(errText(e));
            return true;
          }
          menu.setSections(await buildSections(next));
          return true;
        },
      })),
    );
    if (recents.length) {
      sections.push({ title: "Recent projects", items: recents });
    }
    return sections;
  };

  menu = openCommandMenu({
    sections: await buildSections(await fetchList()),
    placeholder: "Search projects and actions...",
    preselect: mode === "open" || mode === "create" ? mode : undefined,
  });
  activeMenu = menu;
};
