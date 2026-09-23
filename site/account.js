import { $, call, fmtDate, message, session, show } from "/api.js";

const status = $("#status");
const say = (text, kind = "error") => message(status, text, kind);

// ── Busy buttons: disabled + a short label while the server works ──────────
async function busy(button, label, work) {
  const original = button.textContent;
  button.disabled = true;
  button.textContent = label;
  try {
    return await work();
  } finally {
    button.disabled = false;
    button.textContent = original;
  }
}

// ── Tabs (instant; arrow keys move between them) ───────────────────────────
const tabs = [$("#tab-email"), $("#tab-code")];
function selectTab(tab) {
  for (const t of tabs) {
    const on = t === tab;
    t.setAttribute("aria-selected", String(on));
    t.tabIndex = on ? 0 : -1;
    show(document.getElementById(t.getAttribute("aria-controls")), on);
  }
}
for (const t of tabs) {
  t.addEventListener("click", () => selectTab(t));
  t.addEventListener("keydown", e => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    const next = tabs[(tabs.indexOf(t) + (e.key === "ArrowRight" ? 1 : tabs.length - 1)) % tabs.length];
    selectTab(next);
    next.focus();
  });
}

// ── Signed out ─────────────────────────────────────────────────────────────
function showSignIn() {
  show($("#dash"), false);
  show($("#signin"));
  const auth = document.querySelector("[data-auth-link]");
  if (auth) auth.textContent = "Sign in";
}

async function signedIn(token) {
  session.set(token);
  say("");
  await loadDash();
}

$("#email-form").onsubmit = async e => {
  e.preventDefault();
  const email = $("#email").value.trim();
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    $("#email").setAttribute("aria-invalid", "true");
    return say("Enter a valid email address.");
  }
  $("#email").removeAttribute("aria-invalid");
  await busy(e.submitter ?? $("#email-form button"), "Sending…", async () => {
    try {
      await call("POST", "/auth/start", { body: { email } });
      $("#sent-to").textContent = email;
      show($("#email-form"), false);
      show($("#verify-form"));
      say("");
      $("#email-code").focus();
    } catch (err) {
      say(err.message);
    }
  });
};

$("#verify-form").onsubmit = async e => {
  e.preventDefault();
  const code = $("#email-code").value.trim();
  if (!/^\d{6}$/.test(code)) return say("Enter the 6-digit code from the email.");
  await busy(e.submitter ?? $("#verify-form button[type=submit]"), "Checking…", async () => {
    try {
      const r = await call("POST", "/auth/verify", { body: { email: $("#email").value.trim(), code } });
      await signedIn(r.token);
    } catch (err) {
      say(err.message);
    }
  });
};

$("#other-email").onclick = () => {
  show($("#verify-form"), false);
  show($("#email-form"));
  $("#email").focus();
};
$("#resend").onclick = () => $("#email-form").requestSubmit();

$("#code-form").onsubmit = async e => {
  e.preventDefault();
  const code = $("#code").value.trim();
  if (!code) return say("Enter your restore code.");
  await busy(e.submitter ?? $("#code-form button"), "Opening…", async () => {
    try {
      const r = await call("POST", "/auth/restore-code", { body: { code } });
      await signedIn(r.token);
    } catch (err) {
      say(err.message);
    }
  });
};

// ── Signed in ──────────────────────────────────────────────────────────────
const LABEL = { paid: "Uploading", uploaded: "Stored", deleted: "Deleted", rejected: "Too large" };

async function loadDash() {
  const token = session.get();
  let me;
  try {
    me = await call("GET", "/api/me", { token });
  } catch (err) {
    if (err.status === 401) {
      session.set(null);
      showSignIn();
      return;
    }
    return say(err.message);
  }
  show($("#signin"), false);
  show($("#dash"));
  $("#who").textContent = me.email ? `Signed in as ${me.email}` : "Signed in with a restore code";
  show($("#claim-panel"), Boolean(me.email));
  show($("#admin-link"), Boolean(me.isAdmin));
  $("#delete-all").textContent = me.email ? "Delete my account and cloud backups" : "Delete these cloud backups";
  $("#delete-text").textContent = me.email
    ? "Deletes all your cloud backups and signs you out everywhere. Your USB backup is not affected."
    : "Deletes the cloud backups for this restore code. Your USB backup is not affected.";
  const auth = document.querySelector("[data-auth-link]");
  if (auth) auth.textContent = "Account";
  await loadBackups();
}

async function loadBackups() {
  show($("#loading"));
  show($("#list"), false);
  show($("#empty"), false);
  try {
    const { backups } = await call("GET", "/api/cloud/backups", { token: session.get() });
    render(backups);
  } catch (err) {
    say(err.message);
  } finally {
    show($("#loading"), false);
  }
}

function render(backups) {
  const list = $("#list");
  list.replaceChildren(...backups.map(row));
  show(list, backups.length > 0);
  show($("#empty"), backups.length === 0);
}

function row(b) {
  const li = document.createElement("li");
  li.className = "backup";

  const when = document.createElement("div");
  when.className = "when";
  when.textContent = `Uploaded ${fmtDate(b.created)}`;
  const badge = document.createElement("span");
  badge.className = `status ${b.status}`;
  badge.textContent = LABEL[b.status] ?? b.status;
  when.append(badge);

  const sub = document.createElement("div");
  sub.className = "sub";
  sub.textContent =
    b.status === "uploaded"
      ? `Deleted automatically on ${fmtDate(b.expires)} · ${b.tier}`
      : b.status === "paid"
        ? `Upload not finished yet · ${b.tier}`
        : b.status === "rejected"
          ? "Larger than the cloud limit, so it was removed"
          : "No longer stored";

  li.append(when);
  if (b.status === "uploaded" || b.status === "paid") {
    const actions = document.createElement("div");
    actions.className = "actions";
    const del = document.createElement("button");
    del.type = "button";
    del.className = "btn btn-danger btn-sm";
    del.textContent = "Delete";
    del.onclick = () => confirmDelete(li, b.id, del);
    actions.append(del);
    li.append(actions);
  }
  li.append(sub);
  return li;
}

function confirmDelete(li, id, trigger) {
  if (li.querySelector(".confirm")) return;
  const box = document.createElement("div");
  box.className = "confirm";
  const text = document.createElement("span");
  text.textContent = "Delete this cloud backup now? Your USB backup is not affected.";
  const yes = document.createElement("button");
  yes.type = "button";
  yes.className = "btn btn-solid-danger btn-sm";
  yes.textContent = "Delete";
  const no = document.createElement("button");
  no.type = "button";
  no.className = "btn btn-quiet btn-sm";
  no.textContent = "Keep it";
  no.onclick = () => {
    box.remove();
    trigger.focus();
  };
  yes.onclick = () =>
    busy(yes, "Deleting…", async () => {
      try {
        await call("POST", `/api/cloud/backups/${id}/delete`, { token: session.get() });
        say("The cloud backup is deleted.", "ok");
        await loadBackups();
      } catch (err) {
        say(err.message);
      }
    });
  box.append(text, yes, no);
  li.append(box);
  no.focus();
}

$("#claim-form").onsubmit = async e => {
  e.preventDefault();
  const code = $("#claim-code").value.trim();
  if (!code) return say("Enter the restore code of the backup to link.");
  await busy(e.submitter ?? $("#claim-form button"), "Linking…", async () => {
    try {
      const r = await call("POST", "/api/account/claim", { token: session.get(), body: { code } });
      say(r.linked ? "Linked. That backup now belongs to this account." : "That backup was already linked to this account.", "ok");
      $("#claim-code").value = "";
      await loadBackups();
    } catch (err) {
      say(err.message);
    }
  });
};

$("#signout").onclick = async () => {
  await call("POST", "/auth/signout", { token: session.get() }).catch(() => {});
  session.set(null);
  say("You're signed out.", "ok");
  showSignIn();
};

$("#delete-all").onclick = () => {
  show($("#delete-confirm"));
  $("#delete-no").focus();
};
$("#delete-no").onclick = () => {
  show($("#delete-confirm"), false);
  $("#delete-all").focus();
};
$("#delete-yes").onclick = () =>
  busy($("#delete-yes"), "Deleting…", async () => {
    try {
      await call("POST", "/api/account/delete", { token: session.get() });
      session.set(null);
      show($("#delete-confirm"), false);
      say("Everything is deleted. Your USB backup is not affected.", "ok");
      showSignIn();
    } catch (err) {
      say(err.message);
    }
  });

// ── Start ──────────────────────────────────────────────────────────────────
call("GET", "/ready")
  .then(r => {
    if (!r.emailSignIn) {
      show($("#tabs"), false);
      show($("#email-off"));
      selectTab($("#tab-code"));
    }
  })
  .catch(err => say(err.message));

if (session.get()) loadDash();
else showSignIn();
