// On every page: current-page marker, Sign in / Account label, the theme menu,
// and the cursor spotlight on tiles.
import { session } from "/api.js";

const root = document.documentElement;
const reduce = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

// ── Navigation ───────────────────────────────────────────────────────────────
const clean = p => p.replace(/\.html$/, "").replace(/\/$/, "") || "/";
const here = clean(location.pathname);
for (const a of document.querySelectorAll(".nav a:not(.btn)")) {
  const url = new URL(a.href);
  if (!url.hash && clean(url.pathname) === here) a.setAttribute("aria-current", "page");
}
const auth = document.querySelector("[data-auth-link]");
if (auth && session.get()) auth.textContent = "Account";

// ── Theme: System / Light / Dark ─────────────────────────────────────────────
const menu = document.querySelector("details.theme");
if (menu) {
  const current = () => root.dataset.theme ?? "system";
  const mark = () => {
    for (const b of menu.querySelectorAll("[data-theme-set]")) {
      b.setAttribute("aria-pressed", String(b.dataset.themeSet === current()));
    }
  };
  const apply = choice => {
    if (choice === "system") delete root.dataset.theme;
    else root.dataset.theme = choice;
    try {
      if (choice === "system") localStorage.removeItem("ferry-theme");
      else localStorage.setItem("ferry-theme", choice);
    } catch {
      /* this visit only */
    }
    mark();
  };

  menu.addEventListener("click", e => {
    const button = e.target.closest("[data-theme-set]");
    if (!button) return;
    const choice = button.dataset.themeSet;
    menu.open = false;
    if (choice === current()) return;
    // A rare, deliberate action: the new theme wipes in as a circle from the
    // menu. Instant where View Transitions are missing or motion is reduced.
    if (!document.startViewTransition || reduce()) return apply(choice);
    const r = menu.querySelector("summary").getBoundingClientRect();
    const x = r.left + r.width / 2;
    const y = r.top + r.height / 2;
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
    const transition = document.startViewTransition(() => apply(choice));
    transition.ready.then(() =>
      root.animate(
        { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
        { duration: 560, easing: "cubic-bezier(0.22, 1, 0.36, 1)", pseudoElement: "::view-transition-new(root)" },
      ),
    );
  });
  document.addEventListener("click", e => {
    if (menu.open && !menu.contains(e.target)) menu.open = false;
  });
  document.addEventListener("keydown", e => {
    if (e.key === "Escape" && menu.open) {
      menu.open = false;
      menu.querySelector("summary").focus();
    }
  });
  mark();
}

// ── Cursor spotlight on tiles (pointer devices only; CSS hides it on touch) ──
for (const tile of document.querySelectorAll(".tile")) {
  tile.addEventListener("pointermove", e => {
    const r = tile.getBoundingClientRect();
    tile.style.setProperty("--mx", `${e.clientX - r.left}px`);
    tile.style.setProperty("--my", `${e.clientY - r.top}px`);
  });
}
