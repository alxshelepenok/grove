const ID_KEY_RE = /^([A-Z])-(\d+)$/;

const idKey = (id) => {
  const m = ID_KEY_RE.exec(id);
  if (m) {
    const n = Number(m[2]);
    if (Number.isSafeInteger(n)) {
      return [m[1].codePointAt(0), n, id];
    }
  }
  const family = id.length > 0 ? id.codePointAt(0) : 0;
  return [family, Number.MAX_SAFE_INTEGER, id];
};

export const compareIds = (a, b) => {
  const ka = idKey(a);
  const kb = idKey(b);
  return ka[0] - kb[0] || ka[1] - kb[1] || (ka[2] < kb[2] ? -1 : ka[2] > kb[2] ? 1 : 0);
};
