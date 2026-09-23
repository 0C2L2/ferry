// Shared by every page: where Ferry's server lives, how its errors look, and
// the signed-in session.
export const API = "https://api.ferryapp.download";

/** Calls the server; throws an Error carrying the server's user-facing message. */
export async function call(method, path, { token, body } = {}) {
  let res;
  try {
    res = await fetch(API + path, {
      method,
      headers: {
        ...(body ? { "content-type": "application/json" } : {}),
        ...(token ? { authorization: `Bearer ${token}` } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
    });
  } catch {
    throw new Error("Ferry's server can't be reached right now. Check your connection and try again.");
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw Object.assign(new Error(data.error || `Error ${res.status}`), { status: res.status });
  return data;
}

export const $ = sel => document.querySelector(sel);

export function show(el, on = true) {
  el.classList.toggle("hidden", !on);
}

export function message(el, text, kind = "error") {
  el.textContent = text;
  el.className = `msg ${kind}`;
  el.setAttribute("role", kind === "error" ? "alert" : "status");
  show(el, Boolean(text));
}

export const fmtDate = ms => (ms ? new Date(ms).toLocaleDateString(undefined, { dateStyle: "medium" }) : "—");

// Storage can throw (private mode, blocked site data): never let that break a page.
function box(storage) {
  return {
    get(k) {
      try {
        return storage().getItem(k);
      } catch {
        return null;
      }
    },
    set(k, v) {
      try {
        v == null ? storage().removeItem(k) : storage().setItem(k, v);
      } catch {
        /* works for this visit only */
      }
    },
  };
}

const local = box(() => localStorage);
/** The signed-in session (30 days on the server) — kept across visits. */
export const session = {
  get: () => local.get("ferry-session"),
  set: token => local.set("ferry-session", token),
};

/** The raw admin token, if used instead of an admin email — this tab only. */
export const adminToken = box(() => sessionStorage);
