import * as THREE from "../vendor/three.module.min.js";

const ACTIVATE_SLOP_PX = 5;

export const wireConeInputs = ({
  section,
  canvas,
  stage,
  entities,
  verticalLinks,
  nodeMeshes,
  rig,
  edges,
  tooltip,
  onActivate,
}) => {
  const raycaster = new THREE.Raycaster();
  const ndc = new THREE.Vector2();
  const pick = (mx, my) => {
    const w = stage.clientWidth;
    const h = stage.clientHeight;
    ndc.set((mx / w) * 2 - 1, -(my / h) * 2 + 1);
    raycaster.setFromCamera(ndc, rig.camera);
    const hits = [
      ...raycaster.intersectObject(nodeMeshes.sphereMesh, false),
      ...raycaster.intersectObject(nodeMeshes.cubeMesh, false),
    ];
    if (!hits.length) return null;
    const hit = hits.reduce((a, b) => (a.distance <= b.distance ? a : b));
    const ids = hit.object === nodeMeshes.sphereMesh ? nodeMeshes.sphereIds : nodeMeshes.cubeIds;
    return entities[ids[hit.instanceId]];
  };

  const surfaceRows = [...section.querySelectorAll(".cone-surface-row")];
  const linkedIdsOf = (node) => {
    const ids = new Set();
    if (!node) return ids;
    if (node.kind === "f") {
      ids.add(node.id);
      for (const l of verticalLinks) {
        if (l.kind === "surface" && l.to === node.id) ids.add(l.from);
      }
    } else {
      for (const l of verticalLinks) {
        if (l.kind === "surface" && l.from === node.id) ids.add(l.to);
      }
    }
    return ids;
  };
  const syncSurfaceRows = (node) => {
    const active = new Set();
    if (node) {
      if (node.kind === "f") active.add(node.id);
      else {
        for (const l of verticalLinks) {
          if (l.kind === "surface" && l.from === node.id) active.add(l.to);
        }
      }
    }
    for (const el of surfaceRows) {
      el.classList.toggle("is-active", active.has(el.dataset.surfaceFile));
    }
  };

  let hovered = null;
  const setHovered = (n, e) => {
    if (n !== hovered) {
      hovered = n;
      nodeMeshes.sync(n, linkedIdsOf(n));
      syncSurfaceRows(n);
      edges.update(n ? n.id : null);
    }
    canvas.classList.toggle("is-pickable", !!n);
    if (n && e) tooltip.show(n, e.offsetX, e.offsetY);
    else tooltip.hide();
  };

  const workEntities = entities.filter((n) => n.kind === "w");
  let keyIndex = -1;
  const onKeyDown = (e) => {
    if (!workEntities.length) return;
    if (e.key === "ArrowRight" || e.key === "ArrowUp") {
      e.preventDefault();
      keyIndex = (keyIndex + 1) % workEntities.length;
      setHovered(workEntities[keyIndex]);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowDown") {
      e.preventDefault();
      keyIndex = (keyIndex - 1 + workEntities.length) % workEntities.length;
      setHovered(workEntities[keyIndex]);
    } else if (e.key === "Enter" || e.key === " ") {
      if (hovered?.kind === "w") {
        e.preventDefault();
        onActivate?.(hovered.id);
      }
    } else if (e.key === "Escape") {
      keyIndex = -1;
      setHovered(null);
    }
  };

  let dragging = null;
  let downAt = null;
  let downNode = null;

  const onPointerDown = (e) => {
    canvas.setPointerCapture(e.pointerId);
    downAt = [e.offsetX, e.offsetY];
    downNode = pick(e.offsetX, e.offsetY);
    const mode = e.button === 2 || e.shiftKey ? "pan" : "rotate";
    dragging = { mode, x: e.offsetX, y: e.offsetY };
    canvas.style.cursor = downNode ? "pointer" : dragging.mode === "pan" ? "move" : "grabbing";
  };

  const onPointerMove = (e) => {
    if (dragging) {
      const dx = e.offsetX - dragging.x;
      const dy = e.offsetY - dragging.y;
      dragging.x = e.offsetX;
      dragging.y = e.offsetY;
      if (dragging.mode === "rotate") rig.rotateBy(dx, dy);
      else rig.panBy(dx, dy);
      return;
    }
    setHovered(pick(e.offsetX, e.offsetY), e);
  };

  const resetDrag = () => {
    dragging = null;
    downAt = null;
    downNode = null;
    canvas.style.cursor = "";
  };

  const onPointerUp = (e) => {
    if (
      downNode?.kind === "w" &&
      downAt &&
      Math.hypot(e.offsetX - downAt[0], e.offsetY - downAt[1]) < ACTIVATE_SLOP_PX
    ) {
      onActivate?.(downNode.id);
    }
    resetDrag();
  };

  const onWheel = (e) => {
    e.preventDefault();
    rig.zoomBy(e.deltaY);
  };

  const onPointerLeave = () => setHovered(null);
  const onContextmenu = (e) => e.preventDefault();

  const onSurfaceRowEnter = (e) => {
    const el = e.target.closest(".cone-surface-row");
    if (!el) return;
    const node = entities.find((n) => n.id === el.dataset.surfaceFile) ?? null;
    nodeMeshes.sync(node, linkedIdsOf(node));
    syncSurfaceRows(null);
    el.classList.add("is-active");
  };
  const onSurfaceRowLeave = (e) => {
    if (!e.target.closest(".cone-surface-row")) return;
    nodeMeshes.sync(hovered, linkedIdsOf(hovered));
    syncSurfaceRows(hovered);
  };

  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", resetDrag);
  canvas.addEventListener("lostpointercapture", resetDrag);
  canvas.tabIndex = 0;
  canvas.setAttribute("aria-label", "Causality cone scene: arrow keys move between work items, Enter opens the packet");
  canvas.addEventListener("keydown", onKeyDown);
  canvas.addEventListener("pointerleave", onPointerLeave);
  section.addEventListener("mouseover", onSurfaceRowEnter);
  section.addEventListener("mouseout", onSurfaceRowLeave);
  canvas.addEventListener("contextmenu", onContextmenu);
  canvas.addEventListener("wheel", onWheel, { passive: false });

  return {
    get hovered() {
      return hovered;
    },
    dispose() {
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", resetDrag);
      canvas.removeEventListener("lostpointercapture", resetDrag);
      canvas.removeEventListener("keydown", onKeyDown);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      section.removeEventListener("mouseover", onSurfaceRowEnter);
      section.removeEventListener("mouseout", onSurfaceRowLeave);
      canvas.removeEventListener("contextmenu", onContextmenu);
      canvas.removeEventListener("wheel", onWheel);
    },
  };
};
