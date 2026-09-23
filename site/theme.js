// Loaded as a plain (blocking) script in <head> so the saved theme applies
// before the first paint — no flash of the wrong colours.
try {
  const t = localStorage.getItem("ferry-theme");
  if (t === "light" || t === "dark") document.documentElement.dataset.theme = t;
} catch {
  /* storage blocked: follow the system setting */
}
