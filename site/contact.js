// Contact page: the form posts to Ferry's server, which emails the support
// inbox with Reply-To set to the visitor. On success the ferry sails the
// letter off to the lighthouse and the form gives way to a confirmation.
import { $, call, message, show } from "/api.js";

const form = $("#contact-form");
const email = $("#c-email");
const text = $("#c-message");
const count = $("#c-count");
const status = $("#status");
const sent = $("#sent");
const harbor = $(".harbor");
const label = form.querySelector(".send span");
const reduce = matchMedia("(prefers-reduced-motion: reduce)");

// The message box suggests what helps most for each topic.
const hints = {
  "A question": "Ask away — nothing is too basic.",
  "Something went wrong": "What happened, and at which step? Your Windows version helps too.",
  "Cloud copy or account": "The email you used or your backup ID helps. Never your restore code.",
  "Something else": "Ideas, feedback, anything.",
};
const topic = () => form.querySelector("input[name=topic]:checked").value;
const hint = () => (text.placeholder = hints[topic()]);
form.addEventListener("change", e => e.target.name === "topic" && hint());
hint();

text.addEventListener("input", () => {
  const n = text.value.length;
  count.textContent = `${n.toLocaleString("en-US")} / 5,000`;
  count.classList.toggle("near", n > 4500);
});
for (const el of [email, text]) el.addEventListener("input", () => el.removeAttribute("aria-invalid"));

form.addEventListener("submit", async e => {
  e.preventDefault();
  message(status, "");
  const bad = !email.checkValidity() ? email : text.value.trim().length < 10 ? text : null;
  if (bad) {
    bad.setAttribute("aria-invalid", "true");
    bad.focus();
    return message(status, bad === email ? "Enter your email so we can reply." : "Write a little more — at least a sentence.");
  }

  const button = form.querySelector(".send");
  button.disabled = true;
  label.textContent = "Sending…";
  try {
    await call("POST", "/contact", {
      body: { email: email.value.trim(), topic: topic(), message: text.value, website: form.website.value },
    });
  } catch (err) {
    // Our side failed: offer the plain address so the message isn't lost.
    const fallback = err.status >= 500 || !err.status ? " You can also email support@ferryapp.download." : "";
    return message(status, err.message + fallback);
  } finally {
    button.disabled = false;
    label.textContent = "Send message";
  }

  $("#sent-to").textContent = email.value.trim();
  harbor.classList.remove("back");
  harbor.classList.add("sent");
  const swap = () => {
    show(form, false);
    show(sent);
    sent.focus();
  };
  // Let the boat get going before the form changes under it.
  reduce.matches ? swap() : setTimeout(swap, 350);
});

$("#again").addEventListener("click", () => {
  form.reset();
  hint();
  text.dispatchEvent(new Event("input"));
  show(sent, false);
  show(form);
  harbor.classList.remove("sent");
  harbor.classList.add("back"); // a fresh boat comes in from the left
  text.focus();
});
