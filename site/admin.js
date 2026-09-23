import { $, adminToken, call, fmtDate, message, session, show } from "/api.js";

const status = $("#status");
const say = (text, kind = "error") => message(status, text, kind);

// An admin email session (shared with the Account page) or the raw token.
let token = null;

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

async function tryOpen(candidate) {
  const o = await call("GET", "/admin/overview", { token: candidate });
  token = candidate;
  show($("#login"), false);
  show($("#dash"));
  show($("#logout"));
  fill(o);
}

async function start() {
  for (const candidate of [adminToken.get("ferry-admin"), session.get()]) {
    if (!candidate) continue;
    try {
      return await tryOpen(candidate);
    } catch (err) {
      if (err.status !== 401) return say(err.message);
    }
  }
  show($("#login"));
}

function fill({ settings: s, stats, backups }) {
  $("#s-stored").textContent = stats.stored;
  $("#s-uploading").textContent = stats.uploading;
  $("#s-deleted").textContent = stats.deleted;

  $("#emailEnabled").checked = s.emailEnabled;
  $("#emailFrom").value = s.emailFrom;
  $("#emailReplyTo").value = s.emailReplyTo ?? "";
  $("#resendApiKey").value = "";
  $("#key-state").textContent = s.resendApiKeySet ? "A key is saved — leave the field empty to keep it." : "No key saved.";
  show($("#remove-key"), s.resendApiKeySet);
  $("#adminEmails").value = (s.adminEmails ?? []).join(", ");
  $("#cloudFree").checked = s.cloudFree;
  $("#retentionDays").value = s.retentionDays;
  $("#maxActiveBackups").value = s.maxActiveBackups;
  $("#perIpDaily").value = s.perIpDaily;

  const rows = $("#rows");
  rows.replaceChildren(...backups.map(tableRow));
  show($("#no-rows"), backups.length === 0);
}

function tableRow(b) {
  const tr = document.createElement("tr");
  for (const [text, mono] of [
    [fmtDate(b.created), false],
    [b.owner, false],
    [b.status, false],
    [b.status === "uploaded" ? fmtDate(b.expires) : "—", false],
    [b.id.slice(0, 8), true],
  ]) {
    const td = document.createElement("td");
    if (mono) td.className = "mono";
    td.textContent = text;
    tr.append(td);
  }
  const td = document.createElement("td");
  if (b.status === "uploaded" || b.status === "paid") {
    const del = document.createElement("button");
    del.type = "button";
    del.className = "btn btn-danger btn-sm";
    del.textContent = "Delete";
    del.onclick = () => {
      if (del.dataset.armed) {
        busy(del, "Deleting…", async () => {
          try {
            await call("POST", `/admin/backups/${b.id}/delete`, { token });
            say("Backup deleted.", "ok");
            await refresh();
          } catch (err) {
            say(err.message);
          }
        });
      } else {
        // Two-step, inline: the first click arms it, the second deletes.
        del.dataset.armed = "1";
        del.className = "btn btn-solid-danger btn-sm";
        del.textContent = "Click again to delete";
      }
    };
    td.append(del);
  }
  tr.append(td);
  return tr;
}

async function refresh() {
  fill(await call("GET", "/admin/overview", { token }));
}

$("#login-form").onsubmit = async e => {
  e.preventDefault();
  const candidate = $("#token").value.trim();
  if (!candidate) return say("Paste the admin token.");
  try {
    await tryOpen(candidate);
    adminToken.set("ferry-admin", candidate);
    say("");
  } catch (err) {
    say(err.message);
  }
};

$("#logout").onclick = () => {
  adminToken.set("ferry-admin", null);
  token = null;
  location.href = "/account";
};

$("#settings").onsubmit = async e => {
  e.preventDefault();
  const patch = {
    emailEnabled: $("#emailEnabled").checked,
    emailFrom: $("#emailFrom").value,
    emailReplyTo: $("#emailReplyTo").value,
    adminEmails: $("#adminEmails").value,
    cloudFree: $("#cloudFree").checked,
    retentionDays: Number($("#retentionDays").value),
    maxActiveBackups: Number($("#maxActiveBackups").value),
    perIpDaily: Number($("#perIpDaily").value),
  };
  const key = $("#resendApiKey").value.trim();
  if (key) patch.resendApiKey = key;
  await busy(e.submitter ?? $("#settings button[type=submit]"), "Saving…", async () => {
    try {
      await call("POST", "/admin/settings", { token, body: patch });
      say("Settings saved.", "ok");
      await refresh();
    } catch (err) {
      say(err.message);
    }
  });
};

$("#remove-key").onclick = async () => {
  try {
    await call("POST", "/admin/settings", { token, body: { resendApiKey: "", emailEnabled: false } });
    say("Key removed and email switched off.", "ok");
    await refresh();
  } catch (err) {
    say(err.message);
  }
};

$("#test-send").onclick = () =>
  busy($("#test-send"), "Sending…", async () => {
    try {
      await call("POST", "/admin/email-test", { token, body: { to: $("#test-to").value } });
      say(`Test email sent to ${$("#test-to").value}.`, "ok");
    } catch (err) {
      say(err.message);
    }
  });

start();
